# Fix brief incular-widgets-12

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/dx/motion/node.rs`. Other agents are editing other files at
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
### A214-05 - crates/incular-widgets/src/dx/motion/node.rs:231 (§3, low)
Evidence:
```
for (index, value) in mixed.iter_mut().enumerate() {
    *value = base[index] + (tinted[index] - base[index]) * amount;
}
```
Suggested fix: Zip the arrays instead of indexing three of them (assumes `to_matrix()` returns a 20-element array, as `mixed` does):
for ((value, &base), &tinted) in mixed.iter_mut().zip(&base).zip(&tinted) {
    *value = base + (tinted - base) * amount;
}

### A214-04 - crates/incular-widgets/src/dx/motion/node.rs:496 (§11, low)
Evidence:
```
for prop in Prop::ALL {
    let used = self.spec.borrow().poses().any(|p| p.sets(prop));
    if !used {
```
Suggested fix: Compute the used-property set once per resolve instead of rescanning every pose for each of the PROP_COUNT props (same scan repeats in `settle`, line 534):
let mut used = [false; PROP_COUNT];
for pose in self.spec.borrow().poses() {
    for prop in pose.props() {
        used[prop.index()] = true;
    }
}
// then: if !used[prop.index()] { continue; }

### A214-01 - crates/incular-widgets/src/dx/motion/node.rs:654 (§2, high)
Evidence:
```
let values: Vec<MotionValue> = self.values.borrow().iter().flatten().cloned().collect();
for value in values {
    changed |= value.tick(now);
```
Suggested fix: Tick without collecting. Clone one handle per step, since a tick may re-enter `values`:
for index in 0..PROP_COUNT {
    let value = self.values.borrow()[index].clone();
    if let Some(value) = value {
        changed |= value.tick(now);
    }
}

### A215-01 - crates/incular-widgets/src/dx/motion/node.rs:654 (§11, high)
Evidence:
```
let values: Vec<MotionValue> = self.values.borrow().iter().flatten().cloned().collect();
for value in values {
    changed |= value.tick(now);
```
Suggested fix: Iterate the borrow directly instead of collecting a per-frame Vec:
        for value in self.values.borrow().iter().flatten() {
            changed |= value.tick(now);
        }
If MotionValue::tick can re-enter this node (listeners that call resolve/add_extra), keep a reusable scratch buffer on MotionNode instead: take it with std::mem::take, refill with clone(), tick, clear(), and put it back so the capacity is reused across frames.

### A214-02 - crates/incular-widgets/src/dx/motion/node.rs:671 (§2, high)
Evidence:
```
let extras: Vec<_> = self.extras.borrow().clone();
for extra in extras {
    changed |= extra.tick(now);
```
Suggested fix: Avoid the per-frame Vec of Rc clones. Re-borrow per step, because a tick may add a channel and the RefCell must not stay borrowed across the call:
let mut index = 0;
loop {
    let extra = self.extras.borrow().get(index).cloned();
    let Some(extra) = extra else { break };
    changed |= extra.tick(now);
    index += 1;
}

### A215-02 - crates/incular-widgets/src/dx/motion/node.rs:671 (§11, high)
Evidence:
```
let extras: Vec<_> = self.extras.borrow().clone();
for extra in extras {
    changed |= extra.tick(now);
```
Suggested fix: Avoid cloning the extras Vec every frame. extras only change in add_extra (called from build/bind paths), so iterate the borrow:
        for extra in self.extras.borrow().iter() {
            changed |= extra.tick(now);
        }
If a tick can call add_extra, use the same reusable scratch-buffer pattern as the values tick instead.

### A214-03 - crates/incular-widgets/src/dx/motion/node.rs:995 (§2, medium)
Evidence:
```
*self.behaviors.borrow_mut() = spec.behaviors.clone();
for behavior in &spec.behaviors {
    behavior.bind(self, context);
```
Suggested fix: Remove the `behaviors` field (line 117) and its per-build Vec clone. `self.spec` already holds the same Rc<Motion> (set at line 950), so `behaviors()` becomes `self.spec.borrow().behaviors.clone()` and `absorb_offsets` iterates `self.spec().behaviors.iter()` (Rc clone only, no Vec clone, and no RefCell borrow held across `take_offset`).


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
