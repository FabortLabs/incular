# Fix brief incular-devtools-05

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-devtools/src/session.rs`, `tools/incular-devtools/src/property_edit.rs`, `tools/incular-devtools/src/session.rs`, `tools/incular-devtools/src/views/inspection.rs`. Other agents are editing other files at
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
### A036-01 - crates/incular-devtools/src/session.rs:17 (§18, low)
Evidence:
```
#[allow(
    clippy::result_large_err,
    reason = "Tungstenite's handshake callback requires an unboxed HTTP error response"
```
Suggested fix: Change `#[allow(` to `#[expect(` on the same attribute, keeping the reason: `#[expect(clippy::result_large_err, reason = "Tungstenite's handshake callback requires an unboxed HTTP error response")]`. If clippy reports the expectation as unfulfilled, drop the attribute instead.

### A036-03 - crates/incular-devtools/src/session.rs:197 (§2, low)
Evidence:
```
let token = token.clone();
let notify = shutdown_notify.clone();
handshakes.push(async move {
```
Suggested fix: Store the token as shared immutable text: change `ServeArgs.token` from `String` to `Arc<str>` (and the caller that builds ServeArgs from `generate_token()`), then replace the clone with `let token = Arc::clone(&token);`. This removes a String allocation per accepted TCP connection.

### A036-02 - crates/incular-devtools/src/session.rs:342 (§7, low)
Evidence:
```
let text = serde_json::to_string(message).map_err(|_| ())?;
```
Suggested fix: Keep the serde_json::Error (for example via a small private error enum with an Encode(serde_json::Error) variant, plus Timeout and Sink variants, returned by send_message and run_session) so the cause is not dropped. Only do this if a caller would log it; otherwise leave as is.

### A040-02 - tools/incular-devtools/src/property_edit.rs:225 (§2, medium)
Evidence:
```
if is_alignment_grid(&property.options) {
    EditorKind::AlignmentGrid(property.options.clone())
} else if property.options.len() <= SEGMENTED_MAX {
```
Suggested fix: Borrow the options instead of cloning them: make `EditorKind` generic over a lifetime (`pub enum EditorKind<'a>`), change `Segmented`, `Menu` and `AlignmentGrid` to hold `&'a [String]`, and make `pub fn editor_kind(property: &DebugProperty) -> EditorKind<'_>` return `EditorKind::AlignmentGrid(&property.options)`, `EditorKind::Segmented(&property.options)` and `EditorKind::Menu(&property.options)`. Update the call sites that match on these variants so they accept a slice.

### A040-03 - tools/incular-devtools/src/property_edit.rs:320 (§10, medium)
Evidence:
```
self.values
    .entry(key.to_owned())
    .or_insert_with(|| current.clone());
```
Suggested fix: Avoid allocating the key on every call (`entry` needs an owned String, so `to_owned()` runs even when the key is present). Use: `if !self.values.contains_key(key) { self.values.insert(key.to_owned(), current.clone()); }`. The key is allocated and the value cloned only when a property is first recorded.

### A040-04 - tools/incular-devtools/src/property_edit.rs:390 (§10, medium)
Evidence:
```
let slot = self.slots.entry(key.to_owned()).or_default();
let ready = slot
    .last_sent
```
Suggested fix: Same pattern as OriginalValues::record: allocate the key only when the slot is missing. Use: `if !self.slots.contains_key(key) { self.slots.insert(key.to_owned(), Slot::default()); }` followed by `let slot = self.slots.get_mut(key).expect("slot inserted above");`. Each edit offer then no longer allocates a String for an existing property.

### A041-01 - tools/incular-devtools/src/session.rs:68 (§2, low)
Evidence:
```
let display_name = discovery_name(&path);
```
Suggested fix: Remove the eager binding. Build the name only on the warning paths: call discovery_name(&path) inside the format! args at lines 72, 78, 86, 93, 104 and 112. In the Some(false) arm (line 110) call remove_file(&path) and build the name from &path, since path is moved there.

### A041-02 - tools/incular-devtools/src/session.rs:118 (§4, low)
Evidence:
```
report.sessions.sort_by_key(|record| record.started_unix_ms);
```
Suggested fix: report.sessions.sort_unstable_by_key(|record| record.started_unix_ms);

### A041-03 - tools/incular-devtools/src/session.rs:133 (§5, low)
Evidence:
```
Some(std::path::Path::new(&format!("/proc/{}", record.pid)).exists())
```
Suggested fix: Some(std::path::Path::new("/proc").join(record.pid.to_string()).exists())

### A047-03 - tools/incular-devtools/src/views/inspection.rs:508 (§6, low)
Evidence:
```
let model = shared.lock().expect("inspector state");
```
Suggested fix: State the invariant in the message, e.g. `shared.lock().expect("inspector lock is only held for short, non-panicking updates")`, or use the same non-panicking pattern as the neighbouring call sites (lines 238, 313, 527) if poisoning should be tolerated.

### A047-02 - tools/incular-devtools/src/views/inspection.rs:618 (§2, low)
Evidence:
```
Wrap::new(controls.clone()).spacing(4.).run_spacing(4.),
```
Suggested fix: Build the controls row once outside the LayoutBuilder closure: `let controls: Widget = Wrap::new(controls).spacing(4.).run_spacing(4.).into();` before the closure, then inside it use `Padding::new(EdgeInsets::symmetric(8., 0.), controls.clone())`. This removes the Vec<Widget> allocation that the closure makes on every constraint-driven rebuild.

### A047-01 - tools/incular-devtools/src/views/inspection.rs:639 (§2, medium)
Evidence:
```
details_pane(kind, details_data.clone()),
```
Suggested fix: Hoist the pane out of the LayoutBuilder closure, since nothing in it depends on constraints: before `LayoutBuilder::new(...)` add `let pane: Widget = Container::with_child(incular::widgets::internal::ScrollView::vertical(details_scroll.clone(), details_pane(kind, details_data))).alignment(Alignment::TOP_LEFT).color(background()).into();`, then inside the closure replace the `let pane = ...;` statement with nothing and pass `pane.clone()` to `SplitView::horizontal(tree, pane)`. Widget clone is an Rc bump (crates/incular-widgets/src/tree/widget/mod.rs:55), so the Details deep clone (title, subtitle, fields Vec, chips) and the widget rebuild happen once instead of on every constraint change.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
