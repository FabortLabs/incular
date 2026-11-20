# Fix brief incular-scroll-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-scroll/src/extent_index.rs`, `crates/incular-scroll/src/restoration.rs`, `crates/incular-scroll/src/ballistic.rs`, `crates/incular-scroll/src/physics.rs`. Other agents are editing other files at
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
### A153-05 - crates/incular-scroll/src/extent_index.rs:70 (§2, low)
Evidence:
```
let values = values.into_iter().collect::<Vec<_>>();
        let mut tree = Self(vec![0.; values.len()]);
        for (index, value) in values.into_iter().enumerate() {
```
Suggested fix: Avoid the temporary Vec: push the values into the tree first, then run the Fenwick build as a second in-place pass: let mut tree = Self(values.into_iter().collect::<Vec<f64>>()); for cursor in 1..=tree.0.len() { let parent = cursor + cursor.isolate_lowest_one(); if parent <= tree.0.len() { tree.0[parent - 1] += tree.0[cursor - 1]; } }. This keeps one allocation per rebuild_trees call instead of two (rebuild_trees runs on every insert, remove and move).

### A153-03 - crates/incular-scroll/src/extent_index.rs:262 (§2, low)
Evidence:
```
let metrics = Rc::new(ExtentIndexCounters::default());
let _ = metrics.clone();
```
Suggested fix: Delete the line `let _ = metrics.clone();`. It is a dead Rc clone (the clone is dropped immediately and does nothing). Keep the `let metrics = Rc::new(...)` line.

### A153-07 - crates/incular-scroll/src/extent_index.rs:478 (§19, low)
Evidence:
```
let state = self.state.borrow();
        let count = state.len();
        drop(state);
```
Suggested fix: Replace the three lines with: let count = self.len(); (self.len() takes the shared borrow itself and releases it, so the manual borrow and drop are not needed).

### A155-05 - crates/incular-scroll/src/restoration.rs:9 (§1, low)
Evidence:
```
pub(crate) fn restored_scroll_offset(value: serde_json::Value) -> Option<f32> {
```
Suggested fix: Parameter is owned but only read (value.get("offset")). Change to value: &serde_json::Value and update the caller at controller.rs:249 to pass a reference to the value it already holds.

### A155-03 - crates/incular-scroll/src/restoration.rs:14 (§1, medium)
Evidence:
```
pub(crate) fn persist_scroll_offset(restoration: Option<ScrollRestoration>, offset: f32) {
```
Suggested fix: Owned Option<ScrollRestoration> is only read. Change the signature to pub(crate) fn persist_scroll_offset(restoration: Option<&ScrollRestoration>, offset: f32) and update the callers at controller.rs:410 and :505, coordinator.rs:621, activity.rs:286 and :343, physics.rs:851 and :1060 to pass references. This removes the reason for the clones flagged at physics.rs:848 and :1036.

### A155-04 - crates/incular-scroll/src/restoration.rs:21 (§23, medium)
Evidence:
```
.set_json(&restoration.key, serde_json::json!({ "offset": offset }));
```
Suggested fix: Each persisted scroll update builds a serde_json Map and a String key on the input path (apply_physics runs per scroll delta). Persist at most once per frame or on gesture end: record the dirty offset in ScrollState and flush it from the frame boundary, instead of calling set_json per event. If per-event writes are required by the restoration contract, say so in a comment and keep the current behavior.

### U102-02 - crates/incular-scroll/src/ballistic.rs:297 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
```
Suggested fix: BouncingMotion::new takes 7 parameters, which is the default too_many_arguments threshold, so the lint most likely does not fire and the attribute is dead. Remove the attribute if `cargo clippy --workspace --all-targets --all-features -- -D warnings` stays green without it. If clippy still fires, use `#[expect(clippy::too_many_arguments, reason = "mirrors Flutter BouncingScrollSimulation's constructor")]` instead.

### U102-03 - crates/incular-scroll/src/ballistic.rs:443 (§23, medium)
Evidence:
```
self.position + self.distance * (1.0 - (1.0 - t).powf(Self::deceleration_rate()))
(also line 448: self.velocity * (1.0 - t).powf(Self::deceleration_rate() - 1.0))
```
Suggested fix: Store the rate once. Add `rate: f64` to ClampingMotion, set it in `new` from the existing `let rate = Self::deceleration_rate();` (line 424), and use `self.rate` in x (line 443) and dx (line 448, `self.rate - 1.0`). Each clamping sample currently recomputes two ln() calls via deceleration_rate() on every x and dx evaluation, once per frame, with an unchanged value. The computed value is identical, so behavior does not change.

### A154-01 - crates/incular-scroll/src/physics.rs:848 (§2, medium)
Evidence:
```
.then(|| (state.restoration.clone(), result.position))
```
Suggested fix: Make the restoration handle cheap to clone: store `restoration: Option<Rc<ScrollRestoration>>` in `ScrollState` (wrap once where it is set in controller.rs/activity.rs), change `persist_scroll_offset` to take `Option<Rc<ScrollRestoration>>`, and keep this line as `.then(|| (state.restoration.clone(), result.position))`, which then clones only an Rc. Same pattern applies to the clones at physics.rs:1036 and controller.rs:401/501 outside this unit.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
