# Fix brief incular-widgets-11

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/scrolling/sliver_lists.rs`. Other agents are editing other files at
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
### A268-04 - crates/incular-widgets/src/scrolling/sliver_lists.rs:569 (§7, low)
Evidence:
```
if source != destination {
    let _ = self.index.move_item(source, destination);
```
Suggested fix: move_item returns bool and this code assumes success (it then mutates current unconditionally). Make the invariant explicit: let moved = self.index.move_item(source, destination);\n debug_assert!(moved, "reorder source and destination are in range");

### A269-03 - crates/incular-widgets/src/scrolling/sliver_lists.rs:739 (§10, medium)
Evidence:
```
.retain(|item, _| self.order.iter().any(|candidate| candidate == item));
```
Suggested fix: Each widgets entry does a linear scan of self.order every layout, which is O(widgets x items) per frame. Use the inverse index (see the set_child_extent finding) so the check is O(1): `self.widgets.retain(|item, _| self.position_of(*item).is_some());`, or keep a small `Vec<bool>` / bitset of live items rebuilt when order changes.

### A269-01 - crates/incular-widgets/src/scrolling/sliver_lists.rs:740 (§2, high)
Evidence:
```
let order = self.order.clone();
```
Suggested fix: Take the Vec instead of cloning it: `let order = std::mem::take(&mut self.order);` at this spot, and restore it right after `children` is collected: `self.order = order;`. The closure below calls self.child_widget (&mut self) and never reads self.order, so the take is safe and avoids the per-layout clone (the clone exists only to dodge the borrow, §19).

### A269-04 - crates/incular-widgets/src/scrolling/sliver_lists.rs:809 (§11, medium)
Evidence:
```
self.order
.iter()
.position(|candidate| *candidate == item)
```
Suggested fix: Linear search per measured child makes layout O(n^2). Maintain an inverse index `position_of: Vec<usize>` (item id to visual position) updated wherever `order` is written (the reorder completion and the `self.order = next_order` assignment), then `self.position_of.get(item).copied()` here.

### A269-05 - crates/incular-widgets/src/scrolling/sliver_lists.rs:1167 (§2, high)
Evidence:
```
fn snapshot(&self) -> Vec<AnimatedListEntrySnapshot> {
self.state
.borrow()
```
Suggested fix: sync_entries() runs on every perform_layout (line 1296) and allocates a new Vec via snapshot(), then drops the old `self.entries`. Change to `fn snapshot_into(&self, out: &mut Vec<AnimatedListEntrySnapshot>) { out.clear(); out.extend(self.state.borrow().entries.iter().map(|entry| AnimatedListEntrySnapshot { id: entry.id, value: entry.animation.value().clamp(0., 1.), removing: entry.removing, incoming: !entry.removing && entry.animation.is_active() })); }`, and have sync_entries swap two retained buffers (the old `self.entries` becomes the scratch for the next sync) instead of allocating each frame.

### A269-06 - crates/incular-widgets/src/scrolling/sliver_lists.rs:1238 (§10, medium)
Evidence:
```
let live = next.iter().map(|entry| entry.id).collect::<Vec<_>>();
self.base_extents
.retain(|id, _| live.iter().any(|candidate| candidate == id));
```
Suggested fix: Runs on structural changes, not every frame, but the retain is O(keys x entries) with a Vec allocation. Build a set once and test membership in O(1): `let live: std::collections::HashSet<u64> = next.iter().map(|entry| entry.id).collect();` then `self.base_extents.retain(|id, _| live.contains(id));`, and the same for `self.widgets.retain(...)`. (Or retain against `next` directly via a single pass if the set is not needed.)

### A269-07 - crates/incular-widgets/src/scrolling/sliver_lists.rs:1299 (§2, high)
Evidence:
```
let mut positions = self
.index
.materialized_range(cache_start, (cache_end - cache_start).max(0.), 0.)
```
Suggested fix: perform_layout allocates `positions` every frame (`.collect::<Vec<_>>()`). Keep a scratch `positions: Vec<usize>` field: `let mut positions = std::mem::take(&mut self.positions); positions.clear(); positions.extend(self.index.materialized_range(...));`, iterate it with `.iter().copied()` in the children map (child_widget needs &mut self, hence the take), and restore `self.positions = positions;` afterwards. Per-frame allocation in layout per §2/§11.

### A269-08 - crates/incular-widgets/src/scrolling/sliver_lists.rs:1348 (§11, medium)
Evidence:
```
let Some((position, entry)) = self
.entries
.iter()
```
Suggested fix: set_child_extent is called per measured child, and each call scans all entries to find the id (O(n) per child, O(n^2) per layout). Keep an id-to-position map (`HashMap<u64, usize>` or a Vec indexed by a dense slot) rebuilt in sync_entries when the structure revision changes, then look the position up directly.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
