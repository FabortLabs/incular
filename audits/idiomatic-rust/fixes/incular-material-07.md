# Fix brief incular-material-07

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-material/src/catalog/segmented.rs`, `crates/incular-material/src/catalog/search.rs`, `crates/incular-material/src/feedback/dialog.rs`, `crates/incular-material/src/feedback/dialog_handles.rs`, `crates/incular-material/src/inputs/input.rs`, `crates/incular-material/src/inputs/text_selection_toolbar.rs`, `crates/incular-material/src/lists/chips.rs`, `crates/incular-material/src/menus/anchor.rs`, `crates/incular-material/src/navigation/vocabulary.rs`, `crates/incular-material/src/selection/switch.rs`, `crates/incular-material/src/surfaces/components.rs`, `crates/incular-material/src/surfaces/vocabulary.rs`. Other agents are editing other files at
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
### A086-01 - crates/incular-material/src/catalog/segmented.rs:153 (§2, low)
Evidence:
```
let mut children: Vec<Widget> = Vec::new();
for (index, segment) in button.segments.iter().enumerate() {
```
Suggested fix: let mut children: Vec<Widget> = Vec::with_capacity(count);  // count = button.segments.len(), already computed on line 152

### A086-02 - crates/incular-material/src/catalog/search.rs:392 (§2, low)
Evidence:
```
if let Some(hint) = &anchor.view_hint_text {
    header = header.hint_text(hint.clone());
```
Suggested fix: header = header.hint_text(hint);  // hint is &String; SearchBar::hint_text takes impl Into<String> and From<&String> for String exists, so the clone is unnecessary

### A088-01 - crates/incular-material/src/feedback/dialog.rs:406 (§2, medium)
Evidence:
```
let inherited = inherited.cloned().unwrap_or_default();
```
Suggested fix: Borrow instead of cloning the whole theme struct: `let default = ComponentThemeData::default();` then `let inherited = inherited.unwrap_or(&default);`. Every field read below is Copy (Option<f32>, Option<Color>, Option<EdgeInsets>, Option<BorderRadius>, Option<Clip>, Option<Border>), so `inherited.x` reads through the reference without moves. Only `style` (ButtonStyle, heap-carrying) is dropped from the clone.

### A088-02 - crates/incular-material/src/feedback/dialog_handles.rs:91 (§2, medium)
Evidence:
```
let dismiss = self.clone();
        let barrier = ActionSurface::with_child(Container::new().color(self.barrier_color))
```
Suggested fix: Clone only the shared cells the dismiss path needs, not the whole DialogHandle (which deep-clones the Dialog descriptor, including its Option<String> semantic_label and Widget). Move the set_open body into a free fn `fn set_open_cells(open: &Cell<bool>, revision: &Cell<u64>, value: bool)` that does the replace/revision bump, then build the barrier with `let open = Rc::clone(&self.open); let revision = Rc::clone(&self.revision);` and `on_click(move || set_open_cells(&open, &revision, false))`. Minimal alternative: move `let dismiss = self.clone();` inside the `if self.barrier_dismissible` branch (still clones Dialog in the dismissible case).

### A092-01 - crates/incular-material/src/inputs/input.rs:865 (§2, medium)
Evidence:
```
Text::new(label).style(decoration.label_style.clone().unwrap_or_default()),
(also lines 878, 886, 910: prefix_style, suffix_style, counter_style .clone().unwrap_or_default())
```
Suggested fix: `build(self)` owns `decoration` and each of these style fields is read exactly once, so drop the clone: `Text::new(label).style(decoration.label_style.unwrap_or_default())`, and likewise `decoration.prefix_style.unwrap_or_default()`, `decoration.suffix_style.unwrap_or_default()`, `decoration.counter_style.unwrap_or_default()`. Line 899 already does this for error_style, so the code is inconsistent.

### A094-07 - crates/incular-material/src/inputs/text_selection_toolbar.rs:139 (§2, low)
Evidence:
```
Text::new(value.text.clone().unwrap_or_default())
                    .style(
```
Suggested fix: Borrow the stored text: `Text::new(value.text.as_deref().unwrap_or_default())`. Text::new takes impl Into<String>, so the &str works and the Option<String> clone is avoided.

### A095-03 - crates/incular-material/src/lists/chips.rs:280 (§2, low)
Evidence:
```
let label_style = self
    .label_style
    .clone()
    .or(inherited.text_style.clone())
```
Suggested fix: inherited is an owned local (line 245) and text_style is not read again after this line, so move it: .or(inherited.text_style)

### A097-01 - crates/incular-material/src/menus/anchor.rs:311 (§2, medium)
Evidence:
```
let items = if let Some(item_style) = self.item_style.clone() {
ControlButton::with_child(item)
.style(item_style.clone())
```
Suggested fix: Borrow the option and keep the per-item clone that the builder needs: `let items = if let Some(item_style) = self.item_style.as_ref() {` (the inner `.style(item_style.clone())` stays). This removes a whole-ButtonStyle clone on every build that is only read.

### A102-01 - crates/incular-material/src/navigation/vocabulary.rs:1 (§18, low)
Evidence:
```
#![allow(non_upper_case_globals)]
```
Suggested fix: #![expect(non_upper_case_globals, reason = "k_-prefixed constants mirror Flutter's kXxx names")]

### A105-01 - crates/incular-material/src/selection/switch.rs:122 (§2, medium)
Evidence:
```
let value = value.as_ref().clone();
let (theme, selection) = super::resolved_control_theme(context);
```
Suggested fix: Delete `let value = value.as_ref().clone();` and let the body read through the Rc (auto-deref). Replace line 169 `if let Some(callback) = value.on_changed {` with `if let Some(callback) = value.on_changed.clone() {`, which clones only the callback Rc instead of the whole Switch (all other fields are Copy or only borrowed).

### A105-02 - crates/incular-material/src/surfaces/components.rs:241 (§2, medium)
Evidence:
```
let inherited = inherited.cloned().unwrap_or_default();
// (same pattern at line 392 in Divider::build and line 499 in VerticalDivider::build)
```
Suggested fix: Avoid cloning the ~40-field ComponentThemeData just to read it: `let default = ComponentThemeData::default(); let inherited = inherited.unwrap_or(&default);`. Field reads such as `inherited.shape`, `inherited.side`, `inherited.color` are Copy-field reads and keep compiling through the reference. Apply the same change at lines 392 and 499.

### A106-02 - crates/incular-material/src/surfaces/vocabulary.rs:1 (§18, low)
Evidence:
```
#![allow(non_upper_case_globals)]

pub const k_min_interactive_dimension: f32 = K_MIN_INTERACTIVE_DIMENSION;
```
Suggested fix: Replace the module-level allow with an expect that states the reason, since the lint does fire on the lowercase Flutter-parity alias on line 8: #![expect(non_upper_case_globals, reason = "Flutter-parity lowercase alias")];


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
