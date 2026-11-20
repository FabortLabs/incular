# Fix brief incular-material-03

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-material/src/menus/dropdown.rs`, `crates/incular-material/src/app_shell/bars.rs`. Other agents are editing other files at
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
### A099-01 - crates/incular-material/src/menus/dropdown.rs:261 (§2, medium)
Evidence:
```
let value = value.as_ref().clone();
```
Suggested fix: Borrow instead of deep-cloning the whole DropdownButton on every build: `let value: &DropdownButton<T> = value.as_ref();`. Then clone only the fields that are moved out: `if let Some(style) = value.style.clone() {` (line 308), `if let Some(padding) = value.padding` and `dropdown_color`/`alignment` stay as-is (Copy types), and `value.menu_style.clone().unwrap_or_default()` (line 317). The `value.items.clone()` at line 293 stays (PopupMenuButton::from_items takes an owned Vec).

### A099-02 - crates/incular-material/src/menus/dropdown.rs:448 (§2, medium)
Evidence:
```
let items = self.dropdown.items.clone();
if let Some(validator) = validator {
let items = items.clone();
```
Suggested fix: Remove the unconditional clone at line 448. Clone only where a closure needs an owned copy: inside `if let Some(validator) = validator {` use `let items = self.dropdown.items.clone();` and inside `if let Some(callback) = saved {` use `let items = self.dropdown.items.clone();`. Today the Vec is cloned on every register_with_form call even with no validator or on_saved, and twice when a validator is present.

### A099-05 - crates/incular-material/src/menus/dropdown.rs:486 (§2, medium)
Evidence:
```
children.push(Widget::from(value.dropdown.clone()));
```
Suggested fix: `value` is owned here, so move the dropdown instead of cloning the whole DropdownButton (including its items Vec). Compute `let error = ...` (lines 487-492, which reads value.dropdown.value and value.validator) before this line, then use `children.push(Widget::from(value.dropdown));`. Partial moves are fine because DropdownButtonFormField has no Drop impl; value.decoration and value.error_text are still accessible afterwards.

### A099-04 - crates/incular-material/src/menus/dropdown.rs:785 (§2, low)
Evidence:
```
|| entry.label.to_lowercase().contains(&query)
```
Suggested fix: Avoid allocating a lowercased label for every entry on every build. Store a lowercased search key once when the entry is constructed (a private field on DropdownMenuEntry set alongside label) and compare `entry.search_key.contains(&query)`. Keep Unicode `to_lowercase` semantics so behavior is unchanged.

### A099-03 - crates/incular-material/src/menus/dropdown.rs:787 (§3, medium)
Evidence:
```
.collect::<Vec<_>>();
```
Suggested fix: Remove the intermediate `let visible = self.entries.iter().enumerate().filter(...).collect::<Vec<_>>();` (lines 778-787) and chain the filter directly into the map at line 794: `let item_widgets = self.entries.iter().enumerate().filter(|(_, entry)| ...).map(|(index, entry)| { ... }).collect::<Vec<Widget>>();`. The Vec is built once per build and only iterated once.

### A076-01 - crates/incular-material/src/app_shell/bars.rs:281 (§2, medium)
Evidence:
```
row_children.extend(self.actions.clone());
```
Suggested fix: row_children.extend(self.actions.iter().cloned());

### A076-03 - crates/incular-material/src/app_shell/bars.rs:566 (§2, low)
Evidence:
```
let child = Widget::from(self.clone());
if self.pinned && !self.snap {
```
Suggested fix: `child` (a full clone of the SliverAppBar) is unused in the snap-implied FloatingPinned branch (around line 578). Build it lazily only in the branches that use it, e.g. `Widget::from(self.clone())` inline in the pinned/non-pinned-floating/else arms, so that branch does not clone.

### A076-02 - crates/incular-material/src/app_shell/bars.rs:609 (§2, medium)
Evidence:
```
app_bar
.clone()
.toolbar_height(height)
```
Suggested fix: The LayoutBuilder closure runs on every layout pass and clones the whole AppBar (including its Vec of actions) only to override the height. Instead, add a `toolbar_height_override: Option<f32>` parameter to the private `AppBar::build_with_theme` (highest precedence: `override.or(self.toolbar_height).or(inherited.toolbar_height).unwrap_or(K_TOOLBAR_HEIGHT).max(1.0)`), pass `None` from `build`/`From<AppBar>`, and in the closure call `app_bar.build_with_theme(&current_control_theme(context), None, incular_config::TargetPlatform::default(), Some(height))` with no clone. Keep the same platform/inherited arguments the closure passes today.

### A076-04 - crates/incular-material/src/app_shell/bars.rs:817 (§2, low)
Evidence:
```
if !self.extend_body {
children.push(bottom_region.clone().with_key("scaffold-bottom-region"));
}
```
Suggested fix: `bottom_region` is only moved at line ~839 under `if self.extend_body`, and cloned here only under `!self.extend_body`; the two are mutually exclusive. Move it into exactly one branch: `if self.extend_body { stack_children.push(...bottom_region...) } else { children.push(bottom_region.with_key("scaffold-bottom-region")) }`, dropping the `.clone()`.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
