# Fix brief incular-material-06

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-material/src/catalog/expansion.rs`, `crates/incular-material/src/catalog/stepper.rs`, `crates/incular-material/src/navigation/bars.rs`, `crates/incular-material/src/selection/checkbox_radio.rs`, `crates/incular-material/src/buttons/common.rs`, `crates/incular-material/src/catalog/banner.rs`, `crates/incular-material/src/catalog/flexible_space.rs`, `crates/incular-material/src/catalog/grid_tile.rs`. Other agents are editing other files at
  the same time.
- No behavior change. Flutter-parity constants and math stay exactly as they are.
- Do not change any `pub` signature, public type or public field. If a fix
  needs that, skip it.
- Re-read the code around each finding first. The line numbers may have
  drifted a little. Apply the fix only if the finding is still correct and
  the fix is clearly behavior-preserving. Otherwise skip it with a reason.
  Skipping is better than a risky edit.
- Keep fixes minimal and idiomatic. Match the surrounding style. For hot-path
  scratch buffers, prefer a reusable field that is cleared and reused. Keep
  comments short.
- Never add `#[cfg(test)]` code under src/. Do not run cargo; the
  orchestrator builds and tests after the wave. Do not commit.

## Findings
### A083-02 - crates/incular-material/src/catalog/expansion.rs:204 (§12, low)
Evidence:
```
let tile = Rc::new(tile);
Widget::stateful_builder(tile.controller.revision.cell(), move |context| {
```
Suggested fix: Drop the Rc and move `tile` into the closure directly: `Widget::stateful_builder(tile.controller.revision.cell(), move |context| { ... })`. stateful_builder already stores the Fn closure behind its own Rc, and the closure only reads `tile` through `&self`, so the extra Rc<ExpansionTile> allocation per build is redundant. Read `tile.controller.revision.cell()` before the move, as it is today.

### A083-01 - crates/incular-material/src/catalog/expansion.rs:378 (§2, low)
Evidence:
```
let mut items: Vec<Widget> = Vec::new();
let count = list.children.len();
```
Suggested fix: let mut items: Vec<Widget> = Vec::with_capacity(list.children.len());  // each panel adds one item, plus at most one gap or divider

### A087-02 - crates/incular-material/src/catalog/stepper.rs:257 (§1, low)
Evidence:
```
fn vertical(stepper: &Rc<Stepper>, theme: &crate::ThemeData) -> Widget {
```
Suggested fix: Change the signature to `fn vertical(stepper: &Stepper, theme: &crate::ThemeData) -> Widget {` and the same for `fn horizontal` at line 324. Call sites `vertical(&stepper, &theme)` keep compiling via deref coercion from `&Rc<Stepper>`, and the body only reads fields and passes `&Stepper` to `tappable`/`controls`.

### A087-01 - crates/incular-material/src/catalog/stepper.rs:261 (§2, low)
Evidence:
```
    let mut rows = Vec::new();
    for (index, step) in stepper.steps.iter().enumerate() {
```
Suggested fix: Line 261: `let mut rows = Vec::with_capacity(count * 2);` (each step pushes exactly two rows). Same pattern at line 327: `let mut header: Vec<Widget> = Vec::with_capacity(count * 2);` (each step pushes one item plus a divider between steps, so count * 2 bounds it).

### A101-01 - crates/incular-material/src/navigation/bars.rs:136 (§2, medium)
Evidence:
```
let inherited = inherited.cloned().unwrap_or_default();
```
Suggested fix: let default = ComponentThemeData::default();
        let inherited = inherited.unwrap_or(&default);

### A101-02 - crates/incular-material/src/navigation/bars.rs:328 (§2, medium)
Evidence:
```
let inherited = navigation
    .as_ref()
    .map(|themes| themes.navigation_bar_theme.clone())
```
Suggested fix: let default = NavigationBarThemeData::default();
        let inherited = navigation
            .as_ref()
            .map_or(&default, |themes| &themes.navigation_bar_theme);

### A103-01 - crates/incular-material/src/selection/checkbox_radio.rs:233 (§2, medium)
Evidence:
```
value.clone().build(
```
Suggested fix: Make the consuming `fn build(self, ...)` (line 149) take `&self` instead, then call `value.build(&theme, ...)` at line 233 with no clone. Inside build, use `self.on_changed.clone()` at line 208 (the Rc clone is cheap) and `self.fill_color.as_ref()` / `self.check_color.as_ref()` at lines 181 and 194 so the Option is not moved out of a borrow. Copy fields (`side`, `shape`, `value`, `enabled`, `tristate`, `is_error`, `autofocus`) can stay as they are. Confirm `StateProperty::resolve` takes `&self`.

### A103-02 - crates/incular-material/src/selection/checkbox_radio.rs:491 (§2, low)
Evidence:
```
let next = Some(value.clone());
```
Suggested fix: Compare without cloning first, and clone only on the accepted path: `if state.borrow().as_ref() != Some(&value) { *state.borrow_mut() = Some(value.clone()); revision.set(revision.get().checked_add(1).expect("revision exhausted")); callback(value); }`. This removes the clone that happens on every unchanged selection.

### A078-01 - crates/incular-material/src/buttons/common.rs:82 (§2, medium)
Evidence:
```
let mut spec = Self::label(kind, label.clone());
spec.icon = Some(icon.into());
spec.semantic_label = Some(label);
```
Suggested fix: Pass the owned label straight through and drop the overwrite, which discards the clone made inside label():
    pub(super) fn icon_label(
        kind: ButtonKind,
        icon: impl Into<Widget>,
        label: impl Into<String>,
    ) -> Self {
        let mut spec = Self::label(kind, label);
        spec.icon = Some(icon.into());
        spec
    }

### A081-01 - crates/incular-material/src/catalog/banner.rs:121 (§2, medium)
Evidence:
```
if single_line {
    row.push(actions.clone());
}
```
Suggested fix: Move `actions` into exactly one place instead of cloning. Before `let mut column = vec![...]`, replace the `if single_line { row.push(actions.clone()); }` block with `let below_actions = if single_line { row.push(actions); None } else { Some(Widget::from(incular_widgets::Align::new(incular_widgets::Alignment::CENTER_RIGHT, actions))) };`, then remove the later `if !single_line { column.push(...) }` block and insert `column.extend(below_actions);` right after the row widget is pushed into `column`, before the divider push. Separate `if` blocks cannot be used for this because the borrow checker does not correlate the two conditions.

### A083-03 - crates/incular-material/src/catalog/flexible_space.rs:104 (§12, low)
Evidence:
```
let bar = Rc::new(bar);
let first_height = Rc::new(std::cell::Cell::new(None::<f32>));
```
Suggested fix: Drop both Rc wrappers: `let first_height = std::cell::Cell::new(None::<f32>);` and move `bar` and `first_height` straight into the `LayoutBuilder::new(move |context, constraints| ...)` closure. LayoutBuilder::new already wraps the Fn closure in its own Rc, and nothing else holds `first_height`, so the Cell can be owned by the closure directly.

### A083-04 - crates/incular-material/src/catalog/grid_tile.rs:257 (§12, low)
Evidence:
```
let selector = Rc::new(selector);
Widget::stateful_builder(selector.controller.revision_cell(), move |context| {
```
Suggested fix: Drop the Rc and move `selector` into the closure: `Widget::stateful_builder(selector.controller.revision_cell(), move |context| { ... })`. The closure only reads `selector` through `&self`, and stateful_builder already holds the Fn closure behind its own Rc, so the extra Rc<TabPageSelector> allocation is redundant.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
