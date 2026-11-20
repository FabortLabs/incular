# Fix brief incular-widgets-07

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/dx/motion/reorder.rs`. Other agents are editing other files at
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
### A216-04 - crates/incular-widgets/src/dx/motion/reorder.rs:470 (§3, medium)
Evidence:
```
let gaps: Vec<f32> = original
.windows(2)
.map(|pair| match axis {
```
Suggested fix: Compute the average without allocating: `let (sum, count) = original.windows(2).fold((0., 0usize), |(sum, count), pair| { let gap = match axis { Flow::Horizontal => pair[1].origin.x - (pair[0].origin.x + pair[0].size.width), _ => pair[1].origin.y - (pair[0].origin.y + pair[0].size.height) }; (sum + gap, count + 1) }); if count == 0 { 0. } else { sum / count as f32 }`. Same summation order, so results are identical. restack runs on every drag step through plan.

### A216-03 - crates/incular-widgets/src/dx/motion/reorder.rs:533 (§2, high)
Evidence:
```
.retain(|item| item.node.strong_count() > 0);
self.items.borrow().clone()
```
Suggested fix: Stop cloning the whole item Vec on every call. Keep the `retain` (it needs borrow_mut), then have `ordered` and `relax`/`arrange` iterate `self.items.borrow().iter()` directly, cloning only the Rc of items they keep (`ordered` already filters by list). If a Ref cannot be held across the animate calls, filter into a scratch buffer stored on Core and clear it instead of allocating. live_items is reached per drag step through ordered, index_of, plan, locate and arrange, so this allocates several times per pointer move.

### A216-07 - crates/incular-widgets/src/dx/motion/reorder.rs:561 (§2, medium)
Evidence:
```
let lists = if session.from_list == session.to_list {
vec![session.to_list]
```
Suggested fix: plan runs on every drag step and allocates `lists`, `out` (no with_capacity), `ordered`, `original`, `arranged`, `sizes` and `origins` for each list. Use a fixed array for `lists` (`[session.to_list, session.from_list]` with a length of 1 or 2), size `out` with `Vec::with_capacity`, and/or keep these scratch Vecs on Core as RefCell buffers that are cleared and reused. Only do this if the drag-step profile shows it matters, since it is a steady-state per-event cost.

### A216-05 - crates/incular-widgets/src/dx/motion/reorder.rs:624 (§2, medium)
Evidence:
```
let flow = flow_of(&others.iter().map(|(_, rect)| *rect).collect::<Vec<_>>());
```
Suggested fix: Collect is only passed as a slice to flow_of. Make flow_of take `impl Iterator<Item = Rect> + Clone` (or pass a precomputed rect slice from a scratch buffer) so the Vec<Rect> is not allocated. flow_of reads rects[0] and calls .all() twice, so rewrite it with `let mut rects = rects; let first = rects.next()?` style or a `first()` on a clone. Called on every locate, i.e. every drag step.

### A216-06 - crates/incular-widgets/src/dx/motion/reorder.rs:648 (§10, medium)
Evidence:
```
let target = plan
.iter()
.find(|(planned, _, _)| Rc::ptr_eq(planned, &item))
```
Suggested fix: arrange scans the whole plan for every live item (O(items x plan) per drag step). Store the target on Item as a `Cell<Offset>`: reset every live item's target to Offset::ZERO, then assign each planned origin in one pass over `plan` (`for (item, rect, origin) in &plan { item.target.set(*origin - rect.origin) }`), and read `item.target.get()` in the loop. finish() has the same find for the dropped item and can read the same field.

### A217-02 - crates/incular-widgets/src/dx/motion/reorder.rs:775 (§2, high)
Evidence:
```
let Some(mut session) = core.session.borrow().clone() else {
            return;
        };
```
Suggested fix: Clone the Session (which clones K) only after the drop slot is known to have changed. Locate on a borrow first: `let guard = core.session.borrow(); let Some(current) = guard.as_ref() else { return }; let (list, to) = core.locate(current); if (list, to) == (current.to_list, current.to) { return; } drop(guard);` then clone and mutate as before. locate does not borrow core.session, so the borrow is safe. Most pointer moves hit the early return, so they then clone nothing.

### A217-01 - crates/incular-widgets/src/dx/motion/reorder.rs:778 (§11, high)
Evidence:
```
let (list, to) = core.locate(&session);
```
Suggested fix: Make locate allocation-free on every pointer move: it currently calls self.ordered(list) (live_items() clones the whole items Vec, filters, collects and sorts) and then collects `others` and a `Vec<Rect>` for flow_of (reorder.rs:616-624). Count/iterate the lists and items directly, or cache per-list ordered rects behind a generation counter bumped on rect/list/item changes, so drag_update allocates nothing in steady state.

### A217-07 - crates/incular-widgets/src/dx/motion/reorder.rs:834 (§2, medium)
Evidence:
```
.borrow()
            .clone()
            .unwrap_or_else(|| "Item".to_owned());
```
Suggested fix: The label is cloned only to read it into format!. Borrow instead: `let label = item.label.borrow(); let label = label.as_deref().unwrap_or("Item");` (scope the guard so it drops before request_rebuild). Apply the same change to the identical clone in nudge_by at lines 977-981, which runs per screen-reader action.

### A217-03 - crates/incular-widgets/src/dx/motion/reorder.rs:876 (§2, high)
Evidence:
```
let (Some(session), Some(geometry)) = (core.session.borrow().clone(), node.geometry.get())
        else {
```
Suggested fix: auto_scroll runs on every drag_update and on every scheduled frame while near an edge, but only needs session.pointer. Replace the Session clone with a copy of the pointer: `let (Some(pointer), Some(geometry)) = (core.session.borrow().as_ref().map(|session| session.pointer), node.geometry.get()) else { return; };` and use `pointer.y` in place of `session.pointer.y` at lines 883-884.

### A217-04 - crates/incular-widgets/src/dx/motion/reorder.rs:949 (§2, medium)
Evidence:
```
let Some(mut session) = core.session.borrow().clone() else {
            return false;
        };
```
Suggested fix: The keyboard path clones the whole Session (and K) on every arrow press, including presses that hit the list boundary and return at line 955. Check the target index through a borrow first (`let to = core.session.borrow().as_ref().map(|s| s.to as isize + step);`), return on out-of-range, and clone only for an in-range move that is then written back at line 959.

### A217-05 - crates/incular-widgets/src/dx/motion/reorder.rs:952 (§2, medium)
Evidence:
```
let count = core.ordered(session.to_list).len();
```
Suggested fix: ordered() builds, sorts and returns a Vec of every item in the list just to take its length. Add `fn count(&self, list: usize) -> usize` on Core that counts live items with rects in that list (keeping the retain of dead items that live_items does) and use it here. The same pattern appears at line 972 (nudge_by) and line 831 (finish), so change those too.

### A217-06 - crates/incular-widgets/src/dx/motion/reorder.rs:1028 (§2, high)
Evidence:
```
*item.key.borrow_mut() = self.key.clone();
        item.list.set(scope.list);
```
Suggested fix: bind runs on every build for every reorder item and always clones the key, even when unchanged. Use `item.key.borrow_mut().clone_from(&self.key);`, which reuses the existing storage for owned keys (same pattern already used for the label at line 1030).


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
