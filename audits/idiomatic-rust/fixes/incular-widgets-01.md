# Fix brief incular-widgets-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/interaction.rs`. Other agents are editing other files at
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
### A280-01 - crates/incular-widgets/src/tree/interaction.rs:44 (§2, medium)
Evidence:
```
let mut members = Vec::with_capacity(candidates.len());
for element in candidates {
    members.push((element, self.input.gesture_arena.add(key, false)));
```
Suggested fix: Trackpad Started arm: the Vec exists only to read members[0].1 afterwards. Add the winner first and drop the Vec:
    let winner_member = self.input.gesture_arena.add(key, false);
    for _ in 1..candidates.len() {
        self.input.gesture_arena.add(key, false);
    }
Add order is unchanged (winner is candidates[0]), and `candidates` is not used after the loop.

### A280-04 - crates/incular-widgets/src/tree/interaction.rs:116 (§2, medium)
Evidence:
```
if let Some(callback) = self
    .gesture_callbacks(element)
    .and_then(|callbacks| callbacks.on_trackpad_gesture)
```
Suggested fix: `gesture_callbacks` (interaction.rs:1701) returns an owned `GestureCallbacks` by cloning the whole set, and this reads one field. It is called per trackpad update and end event (line 116), per ancestor on trackpad start (lines 39-40, 89-90), and per press candidate (line 766). Read the field by reference for the trackpad path:
    let Some(WidgetKind::Gesture { callbacks, .. }) = self.elements.get(element.0).map(|e| e.widget.kind()) else { return };
    if let Some(callback) = callbacks.on_trackpad_gesture.as_ref() { callback(event); }
The Draggable branch of `gesture_callbacks` never sets a trackpad callback, so this is equivalent. Add a `has_trackpad_gesture_callback(element)` helper with the same match for the candidate filters at lines 38-41 and 88-91.

### A280-05 - crates/incular-widgets/src/tree/interaction.rs:309 (§11, medium)
Evidence:
```
let offset_since_last_stop = Rc::new(std::cell::Cell::new(threshold.map(|_| 0.0f32)));
let last_non_stationary = Rc::new(std::cell::Cell::new(None::<Instant>));
```
Suggested fix: `scroll_drag_callbacks` runs on every press that starts a scroll drag and makes about six separate `Rc::new(Cell..)` allocations (lines 309, 310, 367, 296-297 from the caller at 960-963, and the clones for each closure). Put the per-drag cells in one struct held by one Rc, for example `struct DragCells { offset_since_last_stop: Cell<Option<f32>>, last_non_stationary: Cell<Option<Instant>>, remainder: Cell<f32>, carried_velocity: Cell<Option<f32>>, end_velocity: Cell<Option<f32>> }` with `let cells = Rc::new(DragCells { .. });`, and have each closure capture a clone of `cells`. This makes one allocation per press instead of about six.

### A281-01 - crates/incular-widgets/src/tree/interaction.rs:1101 (§2, high)
Evidence:
```
let dispositions = self.input.gesture_arena.entries(key);
```
Suggested fix: Add `GestureArena::disposition(&self, key, member) -> GestureDisposition` in crates/incular-gestures/src/arena.rs that scans the stream in place without collecting (same result as `disposition_for(&entries(key), member)`). Replace the per-event Vec with it: drop line 1101 and change line 1108 to `let disposition = self.input.gesture_arena.disposition(key, candidate.member);`. Disjoint field borrows keep the `active_gestures` borrow valid.

### A280-02 - crates/incular-widgets/src/tree/interaction.rs:1102 (§2, high)
Evidence:
```
let mut accepts = Vec::new();
let mut rejects = Vec::new();
let mut callbacks = Vec::new();
```
Suggested fix: Every pointer move runs this block. `callbacks` is pushed on each accepted move (line 1117), so a drag allocates on every event; `accepts`, `rejects` and `scale_updates` do the same when used. Keep reusable scratch buffers on the input state (for example a `DispatchScratch { accepts, rejects, callbacks, scale_updates }` field on the tree's input struct). At the top take them with `let mut scratch = std::mem::take(&mut self.input.dispatch_scratch);`, call `clear()` on each, and put them back with `self.input.dispatch_scratch = scratch;` before returning. Taking the buffers out avoids borrow conflicts with the `self.input` borrows in the loop.

### A281-03 - crates/incular-widgets/src/tree/interaction.rs:1102 (§11, high)
Evidence:
```
let mut accepts = Vec::new();
let mut rejects = Vec::new();
let mut callbacks = Vec::new();
```
Suggested fix: Keep the four buffers (accepts, rejects, callbacks, scale_updates) as reusable scratch fields on the input state. At the start of this function take each with `std::mem::take`, call `clear()`, and at the end `clear()` and store it back. Taking and restoring keeps capacity across events, so the pushes made on every drag move stop reallocating. Without the restore the capacity is lost, so do not skip it.

### A281-02 - crates/incular-widgets/src/tree/interaction.rs:1227 (§2, high)
Evidence:
```
let handled = !self.input.gesture_arena.entries(key).is_empty();
```
Suggested fix: Add `GestureArena::has_entries(&self, key) -> bool` in crates/incular-gestures/src/arena.rs applying the same filter as `entries` without building the Vec, then write `let handled = self.input.gesture_arena.has_entries(key);`. This runs on every pointer event that reaches the stream, only to test emptiness.

### A281-06 - crates/incular-widgets/src/tree/interaction.rs:1382 (§2, medium)
Evidence:
```
let drivers = std::mem::take(&mut self.input.scroll_flings);
... self.input.scroll_flings.push(driver);
```
Suggested fix: Remove the matching drivers in place instead of rebuilding the Vec: `for driver in self.input.scroll_flings.extract_if(.., |d| &d.controller == controller) { held_velocity = driver.simulation.dx(driver.elapsed); driver.activity.finish(); }`. `Vec::extract_if` is stable (Rust 1.87+) and the MSRV is 1.99. Order of the surviving drivers is preserved and the call stops allocating.

### A281-04 - crates/incular-widgets/src/tree/interaction.rs:1419 (§11, high)
Evidence:
```
let drivers = std::mem::take(&mut self.input.scroll_flings);
... self.input.scroll_flings.push(driver);
```
Suggested fix: Swap with a retained spare buffer instead of taking: keep `fling_spare: Vec<ScrollFlingDriver>` on the input state, then `let mut drivers = std::mem::take(&mut self.input.fling_spare); std::mem::swap(&mut drivers, &mut self.input.scroll_flings);` and iterate `for mut driver in drivers.drain(..)`. At the end, `self.input.fling_spare = drivers;` (empty, capacity kept). Each frame during a fling currently reallocates the driver Vec from zero when the survivors are pushed back.

### A281-07 - crates/incular-widgets/src/tree/interaction.rs:1542 (§2, medium)
Evidence:
```
.map(|attachment| attachment.controller().clone())
.filter(ScrollController::has_pending_ballistic)
```
Suggested fix: Filter before cloning so only the survivors are cloned: `.filter(|attachment| attachment.controller().has_pending_ballistic()).map(|attachment| attachment.controller().clone())`. This runs every frame from `pump_scroll_flings` and clones every attached controller (an Rc and a DependencySource clone) before discarding most of them.

### A281-08 - crates/incular-widgets/src/tree/interaction.rs:1591 (§2, medium)
Evidence:
```
.map(|attachment| attachment.controller().clone())
.filter(|controller| {
```
Suggested fix: Same as the handed-over collect above: filter on `attachment.controller().has_pending_animation() || attachment.controller().has_pending_settle()` first, then `.map(|attachment| attachment.controller().clone())`. Runs every frame from `start_scroll_animations` and clones every attached controller before the predicate.

### A281-05 - crates/incular-widgets/src/tree/interaction.rs:1666 (§11, high)
Evidence:
```
let keys: Vec<GestureArenaKey> = self.input.active_gestures.keys().copied().collect();
```
Suggested fix: This runs every frame while any press is active. Reuse a scratch `Vec<GestureArenaKey>` field: take it, `clear()`, `extend(self.input.active_gestures.keys().copied())`, iterate with `for key in keys.iter().copied()` (the loop calls `&mut self` methods, so the buffer must be detached from self while iterating), and store it back at the end.

### A281-09 - crates/incular-widgets/src/tree/interaction.rs:1703 (§2, medium)
Evidence:
```
WidgetKind::Gesture { callbacks, .. } => Some((**callbacks).clone()),
```
Suggested fix: `callbacks` is a `Box<GestureCallbacks>` (crates/incular-widgets/src/tree/specs.rs:546). Every press clones the whole struct, about 20 `Option<Rc<dyn Fn..>>` fields, per candidate ancestor. Change the spec field to `Rc<GestureCallbacks>` and return `Option<Rc<GestureCallbacks>>` here (`Some(Rc::clone(callbacks))`), so the per-press cost is one refcount increment. Callers at lines 36, 39, 89, 116 and 766 then use the Rc.

### A281-10 - crates/incular-widgets/src/tree/interaction.rs:1710 (§2, low)
Evidence:
```
let update: Rc<dyn Fn(crate::gestures::DragUpdateDetails)> = Rc::new(|_| {});
```
Suggested fix: This allocates a fresh no-op Rc on every call for each draggable candidate on a press. Create the no-op once when the tree is built (a field such as `noop_drag_update: Rc<dyn Fn(DragUpdateDetails)>`) and `Rc::clone` it here.

### A282-01 - crates/incular-widgets/src/tree/interaction.rs:2061 (§11, high)
Evidence:
```
})
                .collect::<Vec<_>>();
```
Suggested fix: Keep the snapshot but reuse a retained buffer, as update_compositor already does with compositor_scratch: add a field such as `nested_jobs: Vec<NestedJob>` to the input state, then in route_nested_remainder do `let mut jobs = std::mem::take(&mut self.input.nested_jobs); jobs.clear(); jobs.extend(bracket.outer.iter().filter(|link| link.axis == bracket.axis).map(|link| Job { ... }));` (move the Job struct to module scope so the field can name it), iterate `for job in jobs.drain(..)`, and restore with `self.input.nested_jobs = jobs;` before returning. Remove the `if jobs.is_empty()` check, since an empty drain is a no-op. Note the early `return`s inside the loop must also restore the buffer, or the scratch is simply rebuilt on the next event.

### A282-02 - crates/incular-widgets/src/tree/interaction.rs:2250 (§2, high)
Evidence:
```
let path: Vec<_> = path
    .into_iter()
    .filter(|(_, target)| target.context_id() == source_context)
```
Suggested fix: Filter in place instead of collecting a second Vec on every pointer move: `let mut path = self.hit_test(position.hit_point).and_then(|render| self.element_for_render(render)).map(|element| self.drag_target_ancestors(element)).unwrap_or_default();` (before the get_mut), then after `let source_context = active.source.context_id();` call `path.retain(|(_, target)| target.context_id() == source_context);`. The per-move allocation in drag_target_ancestors (Vec::new then push at line 2220) is the other per-event Vec; it can be written into a retained scratch buffer if it shows up in a profile.

### A282-06 - crates/incular-widgets/src/tree/interaction.rs:2268 (§2, low)
Evidence:
```
let left = std::mem::take(&mut active.entered);
        active.target = None;
        for (_, target) in left {
```
Suggested fix: Drain instead of taking the Vec, so its capacity is kept for the next `active.entered.push` at line 2278: `active.target = None; for (_, target) in active.entered.drain(..) { target.leave(); }`. `active` is an exclusive borrow of self.input, so the leave callbacks cannot re-enter the tree and the drain is safe. Only target changes reach this path, so the cost is low.

### A282-03 - crates/incular-widgets/src/tree/interaction.rs:2657 (§11, high)
Evidence:
```
            .collect::<Vec<_>>();
        let mut tickers = tickers;
        tickers.sort_by_key(|(render, ..)| render.0);
```
Suggested fix: Collect into a retained buffer, not a new Vec on every busy frame: add a tree field (like compositor_scratch in tree.rs:1349) typed as the same 7-tuple, then `let mut tickers = std::mem::take(&mut self.ticker_scratch); tickers.clear(); tickers.extend(self.tracked.compositor_nodes.iter().filter_map(...));` drop the redundant `let mut tickers = tickers;` rebinding, iterate `for (...) in tickers.drain(..)`, and store `self.ticker_scratch = tickers;` after the loop.

### A282-05 - crates/incular-widgets/src/tree/interaction.rs:2659 (§4, low)
Evidence:
```
let mut tickers = tickers;
        tickers.sort_by_key(|(render, ..)| render.0);
```
Suggested fix: `tickers.sort_unstable_by_key(|(render, ..)| render.0);` Render ids are unique, so the stable merge buffer is unnecessary. This matches the `sort_unstable_by_key` already used in update_compositor (line 2797).

### A282-04 - crates/incular-widgets/src/tree/interaction.rs:2716 (§11, high)
Evidence:
```
            .collect::<Vec<_>>();
        viewports.sort_by_key(|(render, ..)| render.0);
        let mut changed = false;
```
Suggested fix: Same pattern as tick_controllers: use a retained scratch buffer field (e.g. `viewport_scratch: Vec<(RenderObjectId, ElementId, ScrollDelegate)>`), `take` it, `clear` and `extend` it, iterate with `drain(..)` while calling self.mark_render_dirty, then put it back. The collect runs on every busy frame that contains an animating sliver viewport, so the allocation repeats for the whole animation. The Rc clone of config.delegate is intended and stays.

### A282-07 - crates/incular-widgets/src/tree/interaction.rs:2901 (§2, low)
Evidence:
```
self.layout_render(_render, constraints)?;
```
Suggested fix: The compositor visit list is taken from `compositor_scratch` at line 2783 and only put back at line 2314 (end of the function). When the `?` at this line returns early, the buffer is dropped and is empty on the next frame, so it reallocates. Restore it before propagating, for example by moving the loop body into a helper or by matching the result: `if let Err(error) = self.layout_render(_render, constraints) { self.compositor_scratch = nodes; return Err(error); }`. This only affects the error path.

### A283-02 - crates/incular-widgets/src/tree/interaction.rs:2936 (§11, medium)
Evidence:
```
frame.geometry(self.frame_geometry(_render));
```
Suggested fix: frame_geometry (crates/incular-widgets/src/tree/layout.rs:604-613) allocates `let mut viewports = Vec::new();` on every call that finds a scroll ancestor, and this site runs once per frame per ticking AnimationTicker that wants geometry. Reuse a scratch Vec stored on the tree (take/clear/restore) inside frame_geometry, or have it fill a caller-provided buffer.

### A283-01 - crates/incular-widgets/src/tree/interaction.rs:3439 (§11, medium)
Evidence:
```
let mut viewports = Vec::new();
loop {
    let render = self
```
Suggested fix: Keep a reusable scratch buffer on the tree (e.g. `wheel_viewport_scratch: Vec<(ScrollController, Axis, bool, ScrollPhysics)>`, following the existing `compositor_scratch` pattern): `let mut viewports = std::mem::take(&mut self.wheel_viewport_scratch); viewports.clear();` and restore `self.wheel_viewport_scratch = viewports;` on every return path (including the `viewports.is_empty()` early return and before returning `consumed != 0.`). Each wheel sample over a scroll view currently allocates on the first push.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
