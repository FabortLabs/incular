# Fix brief incular-material-05

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-material/src/catalog/about.rs`, `crates/incular-material/src/navigation/tabs.rs`, `crates/incular-material/src/selection/slider_range.rs`, `crates/incular-material/src/app_shell/shell.rs`. Other agents are editing other files at
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
### A080-02 - crates/incular-material/src/catalog/about.rs:243 (§2, medium)
Evidence:
```
tile
    .child
    .clone()
```
Suggested fix: let title = tile.child.unwrap_or_else(|| Text::new(format!("About {}", tile.application_name)).into());
// `tile` is owned in `From<AboutListTile>`; moving `child` out is a partial move, and `icon`/`on_tap` are still usable below.

### A080-04 - crates/incular-material/src/catalog/about.rs:358 (§2, medium)
Evidence:
```
let licenses = LicenseRegistry::licenses();
for package in LicenseRegistry::packages() {
    let paragraphs = licenses
```
Suggested fix: Add a crate-private `LicenseRegistry::with_licenses<R>(f: impl FnOnce(&[LicenseEntry]) -> R) -> R` that runs `LICENSES.with(|l| f(&l.borrow()))`, use it in `packages()`, and in the `LicensePage` build closure read the entries via that borrow instead of `licenses()`. The build closure currently clones the whole registry, and `packages()` then clones it again on each rebuild. Keep the borrow short; the closure must not call `add_license`.

### A080-05 - crates/incular-material/src/catalog/about.rs:360 (§3, low)
Evidence:
```
let paragraphs = licenses
    .iter()
    .filter(|entry| entry.packages.contains(&package))
```
Suggested fix: let matching = licenses.iter().filter(|entry| entry.packages.contains(&package));
let count = matching.clone().count();
let body: Vec<Widget> = matching
    .flat_map(|entry| entry.paragraphs.iter())
    .map(|paragraph| {
        Widget::from(Container::new().padding(EdgeInsets::only(16., 8., 16., 0.)).child(
            Text::new(paragraph.clone()).style(text.body_medium.clone()).color(scheme.on_surface),
        ))
    })
    .collect();
// Drops the intermediate `paragraphs` Vec and the second filter pass for `count`; then use `.children(body)` at line 387.

### A102-02 - crates/incular-material/src/navigation/tabs.rs:97 (§2, medium)
Evidence:
```
if let Some(style) = style.clone() {
    text = text.style(match color {
```
Suggested fix: if let Some(style) = style {
    text = text.style(match color {  // `style` is not used in the else-if branch, so moving it is fine

### A102-03 - crates/incular-material/src/navigation/tabs.rs:415 (§2, medium)
Evidence:
```
let children = self.tabs.iter().cloned().enumerate().map(|(index, tab)| {
    let enabled = tab.enabled;
```
Suggested fix: let children = self.tabs.iter().enumerate().map(|(index, tab)| {  // tab is &Tab; content(&self), enabled is Copy, so the whole-Tab clone (including its String) is unneeded

### A102-04 - crates/incular-material/src/navigation/tabs.rs:569 (§2, medium)
Evidence:
```
let mut page_view = PageView::new(children.clone())
    .controller(controller.page_controller())
    .viewport_fraction(viewport_fraction);
```
Suggested fix: Hold the children in a shared handle (Rc<[Widget]> or Rc<Vec<Widget>>) captured by the LayoutBuilder closure and pass the shared handle to PageView so each layout clones an Rc instead of allocating a new Vec of Widgets; or build the PageView once outside the closure and only update the page extent inside it. Requires a PageView API that accepts shared children.

### A104-03 - crates/incular-material/src/selection/slider_range.rs:429 (§2, low)
Evidence:
```
if let Some(label) = value.label.clone() {
    root = root.semantic_value(label);
}
```
Suggested fix: `value` is the owned local clone made at line 405 and is not used for `label` afterwards, so move instead: `if let Some(label) = value.label {` (partial move is fine; later reads of value.enabled and value.on_changed are unaffected).

### A104-01 - crates/incular-material/src/selection/slider_range.rs:858 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
fn range_thumb_widget(
```
Suggested fix: #[expect(clippy::too_many_arguments, reason = "Thumb wiring takes one argument per model, callback and layout input; grouping them into a struct is a separate refactor.")]

### A104-02 - crates/incular-material/src/selection/slider_range.rs:991 (§1, low)
Evidence:
```
    on_start: &Option<Rc<dyn Fn(RangeValues) + 'static>>,
    on_changed: &Option<Rc<dyn Fn(RangeValues) + 'static>>,
    on_end: &Option<Rc<dyn Fn(RangeValues) + 'static>>,
```
Suggested fix: Take `Option<&dyn Fn(RangeValues)>` (and in notify_range at line 1021 take `callback: Option<&dyn Fn(RangeValues)>`). Change the call sites to pass `.as_deref()`, e.g. `handle_range_key(&event, &keyboard_model, thumb, rtl, keyboard_on_start.as_deref(), keyboard_on_changed.as_deref(), keyboard_on_end.as_deref())`, and `notify_range(on_changed.as_deref(), next)` in the body of handle_range_key and in the closures at lines 477-1016 that call notify_range.

### A078-04 - crates/incular-material/src/app_shell/shell.rs:284 (§2, low)
Evidence:
```
let inherited = inherited.cloned().unwrap_or_default();
let height = self
    .height
```
Suggested fix: Read the inherited theme by reference and only build a default when absent, so the whole ComponentThemeData (with its style and text_style payloads) is not cloned on every build:
    let default = ComponentThemeData::default();
    let inherited = inherited.unwrap_or(&default);
All later field reads (Option<Size>, Option<Color>) are Copy and work unchanged through the reference.

### A078-03 - crates/incular-material/src/app_shell/shell.rs:524 (§2, low)
Evidence:
```
let child = value.child.clone();
let controller = value.controller.clone();
```
Suggested fix: value is consumed by From<ScaffoldMessenger>, so move the fields out instead of cloning them (ScaffoldMessenger has no Drop impl):
    let revision = value.controller.revision();
    let child = value.child;
    let controller = value.controller;


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
