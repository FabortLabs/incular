# Fix brief examples-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `examples/studio/views/editor.rs`, `examples/devtools_playground/jank.rs`, `examples/motion/main.rs`, `examples/navigation_showcase/main.rs`, `examples/studio/views/command_palette.rs`. Other agents are editing other files at
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
### A327-01 - examples/studio/views/editor.rs:19 (§2, medium)
Evidence:
```
let documents = state.editor.documents.get();
```
Suggested fix: Read the list through the borrowing accessor instead of cloning the whole Vec<DocumentTab> each build: `state.editor.documents.with(|documents| { ... })` around the body that uses `documents` (the tab loop and `documents.get(active_idx)`). Every closure only needs the signal clones already taken on lines 25-26, so nothing else changes.

### A327-03 - examples/studio/views/editor.rs:24 (§2, low)
Evidence:
```
let mut tab_widgets: Vec<Widget> = Vec::new();
```
Suggested fix: `let mut tab_widgets: Vec<Widget> = Vec::with_capacity(documents.len() + 1);` (one tab per document plus the New File button).

### A327-02 - examples/studio/views/editor.rs:143 (§2, medium)
Evidence:
```
let line_count = doc.controller.text().lines().count().max(1);
let line_numbers = (1..=line_count)
    .map(|n| n.to_string())
```
Suggested fix: `TextEditingController::text()` returns an owned String (crates/incular-text/src/editing.rs:415), so this clones the whole buffer just to count lines. Add a borrowing line-count or `with_text` accessor on the controller and use it here. Then build the gutter in one buffer: `let mut line_numbers = String::with_capacity(line_count * 4); for n in 1..=line_count { if n > 1 { line_numbers.push('\n'); } write!(line_numbers, "{n}").expect("writing to String cannot fail"); }` (with `use std::fmt::Write as _;`). This removes one String per line.

### A318-01 - examples/devtools_playground/jank.rs:66 (§3, low)
Evidence:
```
    let rows = (0..SIDE).map(|row| {
        Row::new((0..SIDE).map(move |column| {
    Column::new(rows.collect::<Vec<Widget>>()).into()
```
Suggested fix: Pin the item type in the closure and pass the iterator straight to the constructor: `let rows = (0..SIDE).map(|row| { Widget::from(Row::new(...)) });` then `Column::new(rows).into()`. Column::new takes impl IntoIterator<Item = impl Into<Widget>>, so the intermediate Vec is only needed for type inference.

### A318-03 - examples/devtools_playground/jank.rs:118 (other, low)
Evidence:
```
let burned = if simulate.get() { busy_wait(25) } else { 0 };
let _ = burned;
```
Suggested fix: Replace with `if simulate.get() { std::hint::black_box(busy_wait(25)); }`. The `let burned` binding plus `let _ = burned;` only exists to keep the busy loop's result alive; black_box states that intent directly.

### A321-03 - examples/motion/main.rs:169 (§2, medium)
Evidence:
```
let open = selected
.get()
.and_then(|id| photos.get().into_iter().find(|p| p.id == id));
```
Suggested fix: Reuse the `list` already read at line 124 (Photo is Copy) instead of cloning the whole Vec a second time per build:
        let open = selected
            .get()
            .and_then(|id| list.iter().find(|p| p.id == id).copied());

### A321-04 - examples/motion/main.rs:187 (§18, low)
Evidence:
```
#[allow(dead_code)]
fn scoped(widget: Widget, scope: &incular::motion::ViewScope) -> Widget {
```
Suggested fix: Replace with an expectation that carries a reason:
#[expect(dead_code, reason = "keeps ViewTransitionExt in scope on every target")]

### A322-01 - examples/navigation_showcase/main.rs:50 (§2, low)
Evidence:
```
let route_count = app_navigator.routes().len();
```
Suggested fix: Navigator::routes() clones the whole Vec<Route> only to read its length. Add a non-cloning accessor in crates/incular-navigation/src/navigator.rs next to routes(): `pub fn len(&self) -> usize { self.state.borrow().routes.len() }`, then use `let route_count = app_navigator.len();` here. The new accessor is an additive public API change and needs a CHANGELOG line.

### A322-02 - examples/navigation_showcase/main.rs:74 (§2, low)
Evidence:
```
current.name,
 app_overlay.entries().len()
 ))
```
Suggested fix: Overlay::entries() returns a cloned Vec<OverlayEntry> and it is called twice per build (here and in the loop at line 147). Hoist once before `let mut layers`: `let entries = app_overlay.entries();`, use `entries.len()` here, and change the loop at line 147 to `for entry in entries {`.

### A326-08 - examples/studio/views/command_palette.rs:15 (§2, low)
Evidence:
```
let all_commands = vec![
        StudioCommand::NewFile,
        StudioCommand::OpenFile,
```
Suggested fix: Use an array instead of a Vec: `let all_commands = [StudioCommand::NewFile, ...];` (StudioCommand is Copy and the loop consumes it by value, so no heap allocation is needed).

### A326-09 - examples/studio/views/command_palette.rs:41 (§2, low)
Evidence:
```
let mut command_items: Vec<Widget> = Vec::new();
    for cmd in all_commands {
```
Suggested fix: Use `let mut command_items: Vec<Widget> = Vec::with_capacity(all_commands.len());` before the loop; the length is known.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
