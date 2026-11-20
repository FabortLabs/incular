# Fix brief incular-gestures-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-gestures/src/focus.rs`, `crates/incular-gestures/src/tap_and_drag.rs`. Other agents are editing other files at
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
### A060-05 - crates/incular-gestures/src/focus.rs:823 (§4, medium)
Evidence:
```
let mut indices: Vec<usize> = (0..candidates.len()).collect();
```
Suggested fix: Directional key press allocates an index Vec and fully sorts it only to read the first element. Replace the collect, sort_by and indices.first().copied() with: (0..candidates.len()).min_by(|&left, &right| key(left).partial_cmp(&key(right)).unwrap_or(std::cmp::Ordering::Equal)). min_by returns the first minimum, the same element a stable sort would place first.

### A061-01 - crates/incular-gestures/src/focus.rs:823 (§2, medium)
Evidence:
```
let mut indices: Vec<usize> = (0..candidates.len()).collect();
indices.sort_by(|&left, &right| {
    key(left)
```
Suggested fix: Replace the Vec and full sort with a single pass, and drop the trailing indices.first().copied(): (0..candidates.len()).min_by(|&left, &right| key(left).partial_cmp(&key(right)).unwrap_or(std::cmp::Ordering::Equal)). min_by returns the first minimum, which is the element a stable sort would put first.

### A060-06 - crates/incular-gestures/src/focus.rs:851 (§2, medium)
Evidence:
```
let mut eligible: Vec<usize> = (0..candidates.len())
```
Suggested fix: directional_search allocates eligible (and a second shared Vec at line 878 that is collected and then assigned over eligible) on every arrow-key search, twice per call when wrapping. Replace the shared Vec with an in-place filter: if eligible.iter().any(|&index| candidates[index].shares_scroll_owner) { eligible.retain(|&index| candidates[index].shares_scroll_owner); }. To remove the remaining allocation, thread a caller-owned scratch Vec<usize> through directional_focus_index (its signature change would be a public API change recorded in the CHANGELOG).

### A061-02 - crates/incular-gestures/src/focus.rs:878 (§2, medium)
Evidence:
```
let shared: Vec<usize> = eligible
    .iter()
    .copied()
```
Suggested fix: Filter in place instead of building a second Vec: if eligible.iter().any(|&index| candidates[index].shares_scroll_owner) { eligible.retain(|&index| candidates[index].shares_scroll_owner); } (same order, same result, no allocation).

### A061-03 - crates/incular-gestures/src/focus.rs:891 (§2, medium)
Evidence:
```
let band: Vec<usize> = eligible
    .iter()
    .copied()
    .filter(|&index| overlaps_cross_axis(candidates[index].rect, current, vertical))
```
Suggested fix: Drop the band Vec and its full sort. Extract the comparator at 898-926 into a closure by_target: |&left: &usize, &right: &usize| -> Ordering, then: let in_band = eligible.iter().copied().filter(|&index| overlaps_cross_axis(candidates[index].rect, current, vertical)); let best = if forward { in_band.min_by(by_target) } else { in_band.max_by(by_target) }; if best.is_some() { return best; }. min_by/max_by pick the same element as first/last of the stable sorted band, so the fallthrough to the outside path is unchanged.

### A061-04 - crates/incular-gestures/src/focus.rs:932 (§4, medium)
Evidence:
```
let mut outside = eligible;
outside.sort_by(|&left, &right| {
    let (rect_a, rect_b) = (candidates[left].rect, candidates[right].rect);
```
Suggested fix: Only the extreme element is used (pick_by_order), so sorting the whole list is wasted work. Extract the comparator at 933-975 into by_edge: |&left: &usize, &right: &usize| -> Ordering and return if forward { eligible.iter().copied().min_by(by_edge) } else { eligible.iter().copied().max_by(by_edge) }. max_by returns the last maximum, matching the last element of the stable sort that pick_by_order takes for backward search.

### A060-01 - crates/incular-gestures/src/focus.rs:1037 (§2, medium)
Evidence:
```
|left, right| match (left.focus_order(), right.focus_order()) {
```
Suggested fix: Compare through borrows inside the comparator so no Option<FocusOrder> (String clone for lexical orders) is built per comparison: nodes.sort_by(|left, right| { let left = left.state.traversal_order.borrow(); let right = right.state.traversal_order.borrow(); match (left.as_ref(), right.as_ref()) { (Some(left), Some(right)) => left.compare(right), (Some(_), None) => std::cmp::Ordering::Less, (None, Some(_)) => std::cmp::Ordering::Greater, (None, None) => std::cmp::Ordering::Equal } });

### A060-07 - crates/incular-gestures/src/focus.rs:1318 (§2, low)
Evidence:
```
let nodes: Vec<_> = self
```
Suggested fix: focus_next_silent (Tab path) collects every live node into one Vec, then policy_nodes -> traversable() collects the traversable survivors into a second Vec. Filter once: let live: Vec<FocusNode> = ... keep only nodes passing is_traversable (or make policy_nodes accept the filtered iterator) so only one Vec is built per Tab press.

### A068-02 - crates/incular-gestures/src/tap_and_drag.rs:333 (§10, low)
Evidence:
```
self.kinds.insert(event.pointer, event.kind);
        if self.is_pointer_allowed(&event) {
```
Suggested fix: `kinds` is only ever inserted into and never pruned, so it grows with every distinct pointer id. Add `self.kinds.clear();` at the end of `did_stop_tracking_last_pointer` (no pointer is in progress at that point; `clear` keeps the capacity, so there is no steady-state allocation). Verify no caller reads `kind_for_pointer` after that point.

### A068-03 - crates/incular-gestures/src/tap_and_drag.rs:596 (§23, medium)
Evidence:
```
match self.local_transform.and_then(Transform::inverse) {
            Some(inverse) => inverse.transform_point(end) - inverse.transform_point(end - delta),
```
Suggested fix: The transform only changes in `set_local_transform`, but its inverse is recomputed on every move event (up to twice per `check_drag` and once per `accept_drag`). Add a field `local_inverse: Option<Transform>` (initialised to `None` in `with_axis`), set it in `set_local_transform` as `self.local_inverse = Some(transform).and_then(Transform::inverse)` (or equivalent), and match on `self.local_inverse` here.

### A068-01 - crates/incular-gestures/src/tap_and_drag.rs:855 (§2, medium)
Evidence:
```
for pointer in std::mem::take(&mut self.entries) {
            self.settle(pointer, disposition);
        }
```
Suggested fix: Keep the buffer instead of taking it (take leaves a zero-capacity Vec, so every new gesture regrows `entries` from scratch): `for i in 0..self.entries.len() { let pointer = self.entries[i]; self.settle(pointer, disposition); } self.entries.clear();`. Confirm `settle` never mutates `self.entries`; if it can, move the pointers through a scratch buffer owned by the struct.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
