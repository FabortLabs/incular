# Fix brief incular-gestures-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-gestures/src/velocity.rs`, `crates/incular-gestures/src/force_press.rs`, `crates/incular-gestures/src/multitap.rs`, `crates/incular-gestures/src/arena.rs`, `crates/incular-gestures/src/arena_member.rs`, `crates/incular-gestures/src/multidrag.rs`, `crates/incular-gestures/src/pointer.rs`, `crates/incular-gestures/src/scale.rs`. Other agents are editing other files at
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
### A068-07 - crates/incular-gestures/src/velocity.rs:100 (§2, low)
Evidence:
```
let mut x = Vec::new();
        let mut y = Vec::new();
        let mut time = Vec::new();
```
Suggested fix: The length is bounded by `Self::HISTORY_SIZE`: use `Vec::with_capacity(self.samples.len())` for each, or fixed `[f64; HISTORY_SIZE]` buffers with a count, so the three Vecs are not grown by repeated pushes.

### A068-06 - crates/incular-gestures/src/velocity.rs:182 (§2, low)
Evidence:
```
let a = [
        vec![1.0; m],
        x.to_vec(),
```
Suggested fix: `solve_linear_slope` allocates the design matrix `a` (3 Vecs), `q` (3 Vecs) and the squared-x Vec on every solve, and it runs twice per estimate. `m` is bounded by `HISTORY_SIZE` (20), so use fixed `[f64; 20]` scratch arrays (`[[0.0_f64; VelocityTracker::HISTORY_SIZE]; 3]`) indexed by `[..m]`, or compute the columns inline, instead of `vec!`/`to_vec`/`collect`.

### A068-05 - crates/incular-gestures/src/velocity.rs:194 (§2, medium)
Evidence:
```
let previous = q[i].clone();
                let projection = dot(&q[j], &previous);
```
Suggested fix: The clone exists only to dodge the borrow of `q[j]` against `q[i]`. Split the borrow instead: `let (done, rest) = q.split_at_mut(j); let current = &mut rest[0]; for basis in &done[..j] { let projection = dot(current, basis); for (value, b) in current.iter_mut().zip(basis) { *value -= projection * b; } }` (same arithmetic, no allocation).

### A062-01 - crates/incular-gestures/src/force_press.rs:180 (§2, medium)
Evidence:
```
self.kinds.insert(event.pointer, event.kind);
```
Suggested fix: Delete the write-only `kinds: HashMap<u64, PointerDeviceKind>` field (line 67), its initializer `kinds: HashMap::new(),` (line 98) and this insert. Nothing in force_press.rs reads `self.kinds`, so each pointer-down hashes and grows a map that is never read or pruned.

### A062-02 - crates/incular-gestures/src/force_press.rs:339 (§2, medium)
Evidence:
```
for pointer in std::mem::take(&mut self.entries) {
    self.settle(pointer, disposition);
```
Suggested fix: Keep the buffer so the next press does not reallocate:
    let mut entries = std::mem::take(&mut self.entries);
    for &pointer in &entries {
        self.settle(pointer, disposition);
    }
    entries.clear();
    self.entries = entries;
Verify that settle never pushes to self.entries (such a push would be overwritten by the final assignment).

### A064-03 - crates/incular-gestures/src/multitap.rs:16 (§1, low)
Evidence:
```
fn device_allowed(supported: Option<&Vec<PointerDeviceKind>>, kind: PointerDeviceKind) -> bool {
    supported.is_none_or(|kinds| kinds.contains(&kind))
}
```
Suggested fix: Take a slice: `fn device_allowed(supported: Option<&[PointerDeviceKind]>, kind: PointerDeviceKind) -> bool {`. Update the two callers (lines 130 and 407) to pass `self.supported_devices.as_deref()`.

### A064-01 - crates/incular-gestures/src/multitap.rs:214 (§2, medium)
Evidence:
```
let due: Vec<u64> = self
            .gestures
            .iter()
```
Suggested fix: Drop the snapshot Vec and iterate mutably. The fields `gestures` and `on_long_tap_down` are disjoint, so the borrow checker allows it: `for (pointer, gesture) in self.gestures.iter_mut() { if !gesture.long_tap.is_some_and(|deadline| deadline <= now) { continue; } gesture.long_tap = None; let details = TapDownDetails { global_position: gesture.last.0, local_position: gesture.last.1, kind: gesture.kind }; if let Some(callback) = &self.on_long_tap_down { callback(*pointer, details); } }`. The user callback cannot reach the recognizer, so no re-entrancy. Removes the per-poll collect.

### A058-03 - crates/incular-gestures/src/arena.rs:79 (§8, low)
Evidence:
```
    streams: HashMap<GestureArenaKey, Vec<(GestureArenaMember, bool, GestureDisposition)>>,
```
Suggested fix: Replace the anonymous tuple with a named struct so the `.0`/`.1`/`.2` accesses at lines 104, 116-125, 133-135, 150 read by name: `struct StreamEntry { member: GestureArenaMember, simultaneous: bool, disposition: GestureDisposition }` and store `Vec<StreamEntry>`.

### A058-04 - crates/incular-gestures/src/arena_member.rs:322 (§2, medium)
Evidence:
```
        let requests = std::mem::take(&mut *self.requests.borrow_mut());
```
Suggested fix: `mem::take` leaves an empty `Vec` with no capacity in the shared cell, so the next resolution's push allocates again. Keep a spare buffer: add a `spare: Vec<(u64, GestureDisposition)>` field to `ArenaGestureRecognizer`, then in `decide` do `let spare = std::mem::take(&mut self.spare); let mut requests = std::mem::replace(&mut *self.requests.borrow_mut(), spare);`, iterate `requests.drain(..)`, and restore with `requests.clear(); self.spare = requests;` on every return path (including the early `return GestureDecision::Reject`). The sink's Vec then keeps its capacity across resolutions.

### A064-02 - crates/incular-gestures/src/multidrag.rs:252 (§2, medium)
Evidence:
```
let due: Vec<u64> = self
            .pointers
            .iter()
```
Suggested fix: Avoid the per-poll Vec. `start_drag` needs `&mut self` as a whole, so the snapshot exists to dodge a borrow error (§19). Find one due pointer at a time instead: `while let Some(pointer) = self.pointers.iter().find(|(_, state)| state.timer.is_some_and(|deadline| deadline <= now)).map(|(pointer, _)| *pointer) { let Some(state) = self.pointers.get_mut(&pointer) else { continue; }; state.timer = None; ...existing body... }`. The body clears `timer` first, so each iteration makes progress. The BTreeMap order is unchanged. The find is O(n) per fired timer, and n is tiny. Alternative: a reusable `due: Vec<u64>` field, cleared and refilled in `poll`.

### A066-01 - crates/incular-gestures/src/pointer.rs:737 (§2, high)
Evidence:
```
self.velocity = VelocityTracker::new(self.velocity.kind());
```
Suggested fix: Reset the existing tracker instead of replacing it. Add `pub(crate) fn clear(&mut self) { self.samples.clear(); }` to VelocityTracker in crates/incular-gestures/src/velocity.rs (VecDeque::clear keeps capacity), then in observe_phase Down arm replace lines 737-739 with:
self.velocity.clear();
self.velocity.add_position(event.time, self.local(event.position));

### A067-01 - crates/incular-gestures/src/scale.rs:505 (§11, medium)
Evidence:
```
self.state = ScaleState::Accepted;
self.scale_velocity = Some(VelocityTracker::default());
return false;
```
Suggested fix: Reuse the existing tracker instead of replacing it on every contact change, so its VecDeque buffer is kept. Add a reset method to VelocityTracker in velocity.rs that calls `self.samples.clear()`, then here use `self.scale_velocity.get_or_insert_with(VelocityTracker::default).reset();`. Apply the same change at line 508 (`self.scale_velocity = Some(VelocityTracker::default());` in reconfigure).


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
