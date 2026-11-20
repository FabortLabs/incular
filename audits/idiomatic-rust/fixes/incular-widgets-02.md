# Fix brief incular-widgets-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/raw_input.rs`. Other agents are editing other files at
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
### A293-12 - crates/incular-widgets/src/tree/raw_input.rs:34 (§3, medium)
Evidence:
```
.filter_map(|callbacks| callbacks.on_trackpad_gesture)
.collect::<Vec<_>>();
for callback in callbacks {
```
Suggested fix: The Vec is collected only to be iterated once. Drop the binding and loop over the chain directly: `for callback in self.listener_ids(&self.raw_hit_elements(position)).into_iter().filter_map(|id| self.listener_callbacks(id)).filter_map(|cb| cb.on_trackpad_gesture) { callback(event); }`. The chain borrows only &self, so this is sound.

### A293-11 - crates/incular-widgets/src/tree/raw_input.rs:275 (§2, medium)
Evidence:
```
let factories = self
.raw_input_kind(element)
.and_then(RawInputKind::factory_map)
```
Suggested fix: `.map(ToOwned::to_owned)` clones the full factory slice on every raw-input sync, only to release the borrow of self before mutating self.input.raw_recognizers. Replace with the split-borrow form (read factories from self.elements via a field-level match, then mutate self.input.raw_recognizers), or collect only the `type_id()` values if the factories themselves are not needed after the recognizer loop.

### A293-05 - crates/incular-widgets/src/tree/raw_input.rs:372 (§23, medium)
Evidence:
```
let mut tracked = Vec::new();
let mut current = target.and_then(|render| self.element_for_render(render));
while let Some(candidate) = current {
```
Suggested fix: Called from dispatch_mouse_regions on every mouse event and allocates whenever a focus-behavior detector encloses the pointer. Pass a `&mut Vec<ElementId>` scratch buffer from the caller (or push directly into the enter/exit scratch) instead of returning a fresh Vec.

### A293-02 - crates/incular-widgets/src/tree/raw_input.rs:404 (§2, high)
Evidence:
```
fn listener_ids(&self, elements: &[ElementId]) -> Vec<ElementId> {
let mut seen = HashSet::new();
elements
```
Suggested fix: Avoid a HashSet and an unsized Vec per pointer event (called from dispatch_listeners on every event, from dispatch_pointer_signal and from dispatch_raw_trackpad_gesture). Hit lists are short: `let mut out = Vec::with_capacity(elements.len()); for &id in elements { if !out.contains(&id) && matches!(self.raw_input_kind(id), Some(RawInputKind::Listener { .. })) { out.push(id); } } out`. If the caller can take a reused scratch buffer, return into it instead of allocating.

### A293-01 - crates/incular-widgets/src/tree/raw_input.rs:437 (§2, high)
Evidence:
```
PointerPhase::Move => self
.input
.raw_pointer_routes
```
Suggested fix: Stop cloning the stored route on every Move. Borrow it: `let route: &[ElementId] = self.input.raw_pointer_routes.get(&key).map_or(&[], Vec::as_slice);` for the Move arm (and the Up-with-buttons arm), and iterate `route` directly in the callback loop, calling `self.listener_callbacks(id)` and `self.localized_event(id, event)` per id (both take &self and return owned Rc clones), so no intermediate `callbacks` Vec is built either.

### A293-03 - crates/incular-widgets/src/tree/raw_input.rs:608 (§23, high)
Evidence:
```
let mut next = hit
.raw()
.iter()
```
Suggested fix: Every non-Enter/Exit pointer event allocates `next` here, plus `next.clone()` at line 622 for the map. Keep the per-device hover set in a reusable buffer: swap a scratch Vec in (`let mut next = std::mem::take(&mut self.input.hover_scratch); next.clear(); next.extend(...)`), then `std::mem::swap` it with the stored entry in `self.input.mouse_hover` so the previous buffer becomes the next scratch. Removes both the collect and the clone in steady state.

### A294-01 - crates/incular-widgets/src/tree/raw_input.rs:608 (§23, high)
Evidence:
```
let mut next = hit
.raw()
.iter()
```
Suggested fix: Hoist `next` into a reusable buffer on `self.input` (for example `self.input.hover_scratch: Vec<ElementId>`): take it with `std::mem::take`, `clear()` it, refill it, and return it. The focus-behavior ids from `focus_behavior_elements` (line 614, also a fresh Vec) should be pushed into the same buffer instead of being returned as a new Vec.

### A294-02 - crates/incular-widgets/src/tree/raw_input.rs:622 (§2, high)
Evidence:
```
.insert(key, next.clone())
.unwrap_or_default();
```
Suggested fix: Move `next` into the map instead of cloning it: `let previous = self.input.mouse_hover.insert(key, next).unwrap_or_default();`. Afterwards read the current set back with `self.input.mouse_hover.get(&key)` (or keep the scratch-buffer swap from the previous finding) so no per-event Vec clone happens.

### A293-04 - crates/incular-widgets/src/tree/raw_input.rs:624 (§23, high)
Evidence:
```
let previous_live = previous
.into_iter()
.filter(|id| self.elements.contains(id.0))
```
Suggested fix: dispatch_mouse_regions collects previous_live, exits, enters, exit_callbacks, enter_callbacks, enter_behaviors, exit_behaviors and hover_callbacks, each a Vec, on every mouse move. Walk the borrowed slices instead: exit pass `for id in previous.iter().copied().filter(|id| self.elements.contains(id.0) && !next.contains(id))`, enter pass `for id in next.iter().copied().filter(|id| !previous_live.contains(id))` with the same ordering (exits, then exit behaviors, then enters in reverse, then enter behaviors), and resolve `mouse_region_config` / `focus_behavior_for` inline at call time. Callbacks are owned Rc clones so no self borrow is held across the call.

### A294-03 - crates/incular-widgets/src/tree/raw_input.rs:624 (§23, high)
Evidence:
```
let previous_live = previous
.into_iter()
.filter(|id| self.elements.contains(id.0))
```
Suggested fix: Replace the per-event `previous_live`, `exits` and `enters` Vecs (lines 624-637) with reusable scratch buffers stored on `self.input` (cleared at the start of each pointer event). Compute `exits` and `enters` into those buffers, so the later passes iterate the buffers and no new allocation happens per move event.

### A294-04 - crates/incular-widgets/src/tree/raw_input.rs:638 (§23, high)
Evidence:
```
let exit_callbacks = exits
.iter()
.filter_map(|id| {
```
Suggested fix: The four snapshot Vecs (`exit_callbacks`, `enter_callbacks`, `enter_behaviors`, `exit_behaviors`, lines 638-662) and `hover_callbacks` (lines 676-682) are per-event allocations. Keep the snapshot semantics (callbacks run after all lookups) but store the snapshots in reusable scratch buffers on `self.input` (for example `Vec<Rc<dyn Fn(RawPointerEvent)>>` and `Vec<Rc<FocusBehavior>>`), cleared per event. Do not switch to lazy iterators: a callback may change the tree before later entries are looked up.

### A293-06 - crates/incular-widgets/src/tree/raw_input.rs:689 (§23, high)
Evidence:
```
fn tap_region_registrations(&self, surface: ElementId) -> Vec<TapRegistration> {
self.elements
.iter()
```
Suggested fix: Every primary Down/Up walks the whole element arena once per tap surface, and calls nearest_tap_surface (an ancestor walk) for each element, so the cost is O(elements x depth) per pointer event. Maintain the per-surface tap-region list incrementally (set on mount/update of TapRegion/TextFieldTapRegion, remove on unmount, the same way raw_input_unmounted already cleans routes), or at least walk only the surface's subtree. Also avoids cloning callbacks and group ids into a fresh Vec each press.

### A294-05 - crates/incular-widgets/src/tree/raw_input.rs:690 (§23, medium)
Evidence:
```
self.elements
.iter()
.filter_map(|(raw, element)| {
```
Suggested fix: `tap_region_registrations` scans every element in the tree, and walks each element's ancestor chain via `nearest_tap_surface`, once per tap surface on every primary Down/Up (dispatch_tap_regions line 769). Collect the registrations once per tap event (or cache a per-surface list maintained on mount/unmount/update with a dirty flag), then reuse that list for all surfaces. Confirm the cost with a benchmark before and after.

### A294-06 - crates/incular-widgets/src/tree/raw_input.rs:707 (§2, high)
Evidence:
```
} => (callbacks.clone(), group_id.clone(), *consume_outside_taps),
RawInputKind::TextFieldTapRegion {
```
Suggested fix: `TapRegionGroupId::Named(String)` makes `group_id.clone()` a heap allocation for every tap-region element on every primary Down/Up. Make `TapRegistration` borrow from the element instead: `group_id: Option<&'a TapRegionGroupId>` and `callbacks: &'a TapRegionCallbacks` (lines 20-30 and 689-729). The dispatch code only reads them. Use the same change for the TextFieldTapRegion arm at line 716.

### A294-07 - crates/incular-widgets/src/tree/raw_input.rs:757 (§23, high)
Evidence:
```
let surfaces = hit
.iter()
.copied()
```
Suggested fix: `dispatch_tap_regions` allocates on every primary Down/Up: the `surfaces` Vec (757), a `registrations` Vec per surface (769), `hit_groups` HashSet (773), `inside` Vec (778), `inside_ids` HashSet (788) and `outside` Vec (792). Keep these in reusable scratch buffers on `self.input`, or compute `inside` flags into one reused `Vec<bool>` and check group membership by scanning the small `hit_groups` buffer. Using `&TapRegionGroupId` borrows (see the finding for line 707) removes the HashSet hashing of cloned Strings as well.

### A293-07 - crates/incular-widgets/src/tree/raw_input.rs:773 (§23, medium)
Evidence:
```
let hit_groups = registrations
.iter()
.filter(|registration| hit.contains(&registration.id))
```
Suggested fix: Per surface, per Down/Up, this builds `surfaces` (line 757), `hit_groups` HashSet (773), `inside` Vec (778), `inside_ids` HashSet (788) and `outside` Vec (792). Use a single pass over `registrations` with a per-registration `inside: bool` computed from `hit` and a small group list (`Vec` with `contains`, groups are few), and iterate the two callback sets directly. Reuse one scratch buffer for the surface list if it still allocates.

### A294-08 - crates/incular-widgets/src/tree/raw_input.rs:829 (§2, high)
Evidence:
```
.map_or_else(Vec::new, ToOwned::to_owned)
}

```
Suggested fix: `raw_gesture_factories` allocates and clones the whole factory slice for each element, and it is called per hit element inside the Down loop at line 862. Avoid the Vec: read each factory's `type_id` by index in the loop (for example `factory_count(element)` plus `factory_type_id(element, index)`), or copy only the `TypeId`s into a reusable `Vec<TypeId>` on `self.input`. Borrow `self.elements` and `self.input.raw_recognizers` as disjoint fields to avoid the conflict that forced the clone.

### A293-09 - crates/incular-widgets/src/tree/raw_input.rs:833 (§23, medium)
Evidence:
```
fn raw_gesture_elements(&self, hit: &[ElementId]) -> Vec<ElementId> {
let mut seen = HashSet::new();
hit.iter()
```
Suggested fix: Runs on every primary Down that starts a raw gesture stream and allocates a HashSet plus a result Vec. Hit lists are short: replace the HashSet with `out.contains(&id)` over the output Vec (`Vec::with_capacity(hit.len())`), and only keep elements whose `factory_map()` is non-empty, which is already the filter predicate.

### A294-09 - crates/incular-widgets/src/tree/raw_input.rs:833 (§2, medium)
Evidence:
```
let mut seen = HashSet::new();
hit.iter()
.copied()
```
Suggested fix: `raw_gesture_elements` builds a fresh HashSet on every primary Down. Hit lists are short, so dedupe with a reusable `Vec<ElementId>` scratch buffer on `self.input` and `contains`, or `Vec::with_capacity(hit.len())`. Keep the output buffer reusable too, since its result is only iterated in `dispatch_raw_gesture_stream`.

### A293-08 - crates/incular-widgets/src/tree/raw_input.rs:862 (§2, high)
Evidence:
```
let factories = self.raw_gesture_factories(element);
for factory in factories {
let type_id = factory.type_id();
```
Suggested fix: raw_gesture_factories (line 826) clones the whole factory slice (`map_or_else(Vec::new, ToOwned::to_owned)`) for every element in the hit list on each primary Down. The clone exists only to dodge the &mut self.input borrow. Use split borrows instead: take `let factories = self.elements.get(element.0).and_then(|e| match e.widget.kind() { WidgetKind::RawInput { kind, .. } => kind.factory_map(), _ => None }).unwrap_or(&[]);` and mutate `self.input.raw_recognizers` through the field directly (disjoint from `self.elements`), so no Vec<ErasedGestureRecognizerFactory> is cloned.

### A293-10 - crates/incular-widgets/src/tree/raw_input.rs:901 (§23, high)
Evidence:
```
let active_members = self.input.raw_gesture_streams.get(&key).map(|active| {
active
.members
```
Suggested fix: Every non-Down event on an active raw gesture stream (every Move/Up/Cancel) allocates `active_members` here, and `self.input.gesture_arena.entries(key)` (line 908, also re-called at 950 and 955) allocates a Vec on each call. Iterate the members by index with Copy tuples (`for i in 0..len { let (element, member, type_id) = { let m = &self.input.raw_gesture_streams[&key].members[i]; (m.element, m.member, m.type_id) }; ... }`) and add a non-allocating `GestureArena::disposition(key, member) -> Option<GestureDisposition>` in incular-gestures/src/arena.rs to replace the `entries` snapshot lookups via `disposition_for`.

### A294-10 - crates/incular-widgets/src/tree/raw_input.rs:901 (§23, high)
Evidence:
```
let active_members = self.input.raw_gesture_streams.get(&key).map(|active| {
active
.members
```
Suggested fix: Every Move/Up/Cancel event on an active raw gesture stream allocates `active_members` (line 901) and `dispositions` (line 908, from `GestureArena::entries`, which returns a Vec). Iterate `members` by index without collecting, and replace `disposition_for(&dispositions, member)` with a single-member query such as `self.input.gesture_arena.disposition(key, member)` that does not allocate. Add that query to `incular-gestures` arena.rs next to `entries`.

### A294-11 - crates/incular-widgets/src/tree/raw_input.rs:950 (§23, high)
Evidence:
```
self.apply_raw_arena_entries(key, self.input.gesture_arena.entries(key));
}
for (member, action) in accepts {
```
Suggested fix: `self.input.gesture_arena.entries(key)` is called again inside the reject loop (line 950) and inside the accept loop (line 955: `disposition_for(&self.input.gesture_arena.entries(key), member)`), so each rejected or accepted member allocates a fresh Vec. Use the single-member disposition query from the finding for line 901 in the accept check, and call `entries` at most once per reject batch rather than once per member.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
