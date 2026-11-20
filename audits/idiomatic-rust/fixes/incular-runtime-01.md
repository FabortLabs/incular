# Fix brief incular-runtime-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-runtime/src/frame.rs`. Other agents are editing other files at
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
### A133-04 - crates/incular-runtime/src/frame.rs:140 (§8, low)
Evidence:
```
pointer_inside: bool,
pointer_position_known: bool,
```
Suggested fix: The pair encodes three states; (false, true) never occurs (Enter sets inside=true/known=false, Exit clears both). Replace both fields with one enum, e.g. `enum PointerTracking { Outside, InsideUnknownPosition, InsideAt }`, and update the readers (`!self.pointer_inside || !self.pointer_position_known` in refresh_hover_after_layout and effective_mouse_cursor) to a single match.

### A134-01 - crates/incular-runtime/src/frame.rs:1186 (§2, medium)
Evidence:
```
self.tree
    .set_diagnostic_input_trigger(None, &event, diagnostic_input_trigger);
```
Suggested fix: Every input event is cloned into the recursion-diagnostics state, and only Text(String) and Ime carry heap data, so each keystroke or IME event allocates. Avoid the clone on the input path: have handle_input take the event by value and store a Copy-able trigger summary (pointer phase/position, key data, or a char count for Text/Ime) in recursion.rs instead of InputEvent. Alternatively store the event only while diagnostics are enabled. Keep diagnostic_input_trigger formatting lazy at report time.

### A134-02 - crates/incular-runtime/src/frame.rs:1278 (§14, low)
Evidence:
```
event => {
    match event {
        InputEvent::Scroll { delta } => {
```
Suggested fix: The nested match repeats the Pointer* variants as unreachable!() arms and ends in an early return. Move the non-pointer handling into a helper, e.g. fn handle_non_pointer_input(&mut self, event: InputEvent) -> Option<EventTarget> containing the Scroll/Key/Text/Ime/TrackpadGesture/WindowResized arms, and in the destructuring match write `other => return self.handle_non_pointer_input(other),`. This removes the three unreachable!() arms and one nesting level. No behavior change.

### A135-07 - crates/incular-runtime/src/frame.rs:1998 (§2, medium)
Evidence:
```
let fields = self.tree.focusable_elements();
        if fields.is_empty() {
            return;
```
Suggested fix: `focusable_elements()` (crates/incular-widgets/src/tree/focus.rs:62) allocates a Vec on every Tab press. Add a tree method that returns the neighbour of `self.focused` in traversal order (`focus_neighbor(focused, reverse) -> Option<ElementId>`) or fills a Runtime-owned buffer, so focus_next does no allocation per event.

### A135-02 - crates/incular-runtime/src/frame.rs:2073 (§2, medium)
Evidence:
```
if self.tree.dispatch_keyboard(self.focused, event.clone()) {
                self.frame_requested = true;
                return;
```
Suggested fix: Make `dispatch_keyboard` (crates/incular-widgets/src/tree/focus.rs:110) and `dispatch_keyboard_where` take `&KeyboardEvent`, and have `GestureCallbacks::handle_keyboard` take a reference, so the walk clones nothing. Then call `self.tree.dispatch_keyboard(self.focused, &event)` here. Today every key-down clones the keyboard_types event, which owns a String for character keys, and the ancestor walk clones it again per listener.

### A135-01 - crates/incular-runtime/src/frame.rs:2226 (§2, high)
Evidence:
```
self.tree
            .text_field_input_snapshot(field)
            .is_some_and(|snapshot| snapshot.obscure_text)
```
Suggested fix: Use the text-free accessor: `self.tree.text_field_input_geometry(field).is_some_and(|snapshot| snapshot.obscure_text)` here, and in `text_field_pointer_selection_enabled` (line 2234) replace `text_field_input_snapshot(field)` with `text_field_input_geometry(field)`. Both helpers only read read_only/obscure_text, so the full text copy of the field is wasted on every pointer event. Confirm the geometry snapshot carries those two flags (it is built by the same function with text left empty).

### A135-06 - crates/incular-runtime/src/frame.rs:2647 (§3, low)
Evidence:
```
.collect::<Vec<_>>();
        let mut reactive = self.reactive.borrow_mut();
        for id in ids {
```
Suggested fix: Hot-reload path only. The Vec is collected and iterated once, and nothing else borrows `self.builders` mutably in between, so iterate the chain directly: `let mut reactive = self.reactive.borrow_mut(); for id in root.into_iter().chain(self.builders.keys().copied().filter(|id| Some(*id) != root)) { ... }` and delete the `let ids` binding.

### A135-03 - crates/incular-runtime/src/frame.rs:2767 (§11, medium)
Evidence:
```
let placements = self.tree.platform_view_placements();
        for record in self.platform_views.values_mut() {
            record.mounted = false;
```
Suggested fix: `platform_view_placements()` (crates/incular-widgets/src/tree/retained.rs:1000) returns a fresh Vec every frame whenever a platform view is mounted. Have the tree fill a buffer owned by Runtime (e.g. `platform_view_placements_into(&self, out: &mut Vec<PlatformViewPlacement>)`), taking it with `std::mem::take` at the top of this fn and putting it back at the end so the vector is reused.

### A135-05 - crates/incular-runtime/src/frame.rs:2799 (§10, medium)
Evidence:
```
for record in self.platform_views.values_mut() {
            if record.mounted {
                continue;
```
Suggested fix: `platform_views` is a HashMap, so the order in which hidden Place commands reach `platform_view_commands` changes from run to run. Switching the map to BTreeMap (see the collect_platform_view_commands finding) fixes this with the same change.

### A135-04 - crates/incular-runtime/src/frame.rs:2828 (§2, medium)
Evidence:
```
let mut ids: Vec<_> = self.platform_views.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
```
Suggested fix: Change `platform_views` (field at line 170, constructed at line 330) to `BTreeMap<PlatformViewId, PlatformViewRecord>`; PlatformViewId already derives Ord. Then iterate in order without the Vec or the sort: `for record in self.platform_views.values_mut() { self.platform_view_commands.extend(record.controller.take_commands()); }`. Drop the `keys()` lookup as well.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
