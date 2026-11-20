# Fix brief incular-widgets-22

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/devtools_props.rs`, `crates/incular-widgets/src/drag_drop.rs`, `crates/incular-widgets/src/dx/motion/dragging.rs`, `crates/incular-widgets/src/dx/motion/geometry.rs`, `crates/incular-widgets/src/dx/motion/lint.rs`, `crates/incular-widgets/src/dx/motion/presence.rs`. Other agents are editing other files at
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
### A207-02 - crates/incular-widgets/src/devtools_props.rs:465 (§2, medium)
Evidence:
```
let old = inspect_properties(old)
    .into_iter()
    .map(|property| (property.name, property.value))
```
Suggested fix: diff_properties runs on every compatible element update when the devtools feature is on, and it materializes two full DebugProperty lists (a String per property name, plus the option vectors and string values) and two BTreeMaps and a BTreeSet per call. Make the property names &'static str (or Cow<'static, str>) in incular-devtools-protocol and have diff_properties walk the curated properties through a visitor or a reused scratch buffer, so the unchanged path allocates nothing. Alternatively, skip the diff entirely when the old and new kinds are equal.

### A207-01 - crates/incular-widgets/src/devtools_props.rs:478 (§2, high)
Evidence:
```
let before = old.get(name).cloned();
let after = new.get(name).cloned();
(before != after).then(|| PropertyChange {
```
Suggested fix: Compare by reference and clone only the changed survivors:

    .into_iter()
    .filter(|name| old.get(*name) != new.get(*name))
    .take(32)
    .map(|name| PropertyChange {
        name: name.clone(),
        old: old.get(name).cloned(),
        new: new.get(name).cloned(),
    })
    .collect()

### A210-03 - crates/incular-widgets/src/drag_drop.rs:453 (§11, high)
Evidence:
```
let placed: Widget =
    Positioned::new(IgnorePointer::new(feedback.child.clone()))
 Widget::from(Stack::new(vec![placed]))
```
Suggested fix: The feedback overlay subtree is rebuilt on every pointer move during a drag (DragSourceBinding::update bumps revision whenever position changes). Each rebuild allocates a Vec for Stack::new(vec![placed]), clones the feedback child and the portal child, and reconstructs the Positioned/IgnorePointer nodes. Keep the feedback position out of the build path (for example a shared Cell/ offset read by the positioned layer, or a layout-time offset), so only the overlay's layout is refreshed per move, and build the Stack once per drag start.

### A210-01 - crates/incular-widgets/src/drag_drop.rs:638 (§2, high)
Evidence:
```
fn update(&self, offset: Offset) {
    if let (Some(payload), Some(callback)) = (self.payload(), &self.on_update) {
```
Suggested fix: fn update(&self, offset: Offset) {
    let Some(callback) = &self.on_update else {
        return;
    };
    if let Some(payload) = self.payload() {
        callback(payload, offset);
    }
}
// Same shape for leave(): check on_leave first, then clone the payload.

### A212-02 - crates/incular-widgets/src/dx/motion/dragging.rs:195 (§2, high)
Evidence:
```
fn drag_of(node: &MotionNode) -> Option<Drag> {
    node.spec().drag.as_ref().map(|drag| drag.0.clone())
```
Suggested fix: Return a borrow instead of a clone: `fn drag_of(spec: &Motion) -> Option<&Drag> { spec.drag.as_ref().map(|drag| &drag.0) }`. In `drag_update` and `drag_end` take the snapshot first: `let spec = node.spec(); let Some(drag) = drag_of(&spec) else { return };`. `spec()` is an Rc clone. This removes a deep clone of `Drag` per pointer move, including the `Vec<Offset>` inside `Release::SnapTo`.

### A212-03 - crates/incular-widgets/src/dx/motion/dragging.rs:220 (§2, high)
Evidence:
```
for behavior in node.behaviors() {
    behavior.drag_update(node, global);
}
```
Suggested fix: Stop cloning the behavior list per call. Store behaviors as `RefCell<Rc<[Rc<dyn SiteBehavior>]>>` and have `behaviors()` return the `Rc<[..]>` (a refcount bump). The same change covers the calls at lines 206 and 230. Alternatively, iterate the borrowed slice in place without holding the borrow across the callback.

### A212-05 - crates/incular-widgets/src/dx/motion/geometry.rs:46 (§2, medium)
Evidence:
```
|| node
    .behaviors()
    .iter()
```
Suggested fix: Avoid the Vec clone in `wants`: add `pub(crate) fn any_behavior_wants_geometry(&self) -> bool` on `MotionNode` that iterates `self.behaviors.borrow().iter().any(|b| b.wants_geometry())`, and call it here instead of `node.behaviors().iter().any(...)`. Alternatively, make `behaviors()` return an `Rc<[..]>` as described in the dragging.rs:220 finding.

### A212-04 - crates/incular-widgets/src/dx/motion/geometry.rs:93 (§2, medium)
Evidence:
```
let listeners = node.geometry_listeners.borrow().clone();
for listener in listeners {
    listener.geometry(node, previous, geometry);
```
Suggested fix: Store `geometry_listeners` as `RefCell<Rc<[Rc<dyn GeometryListener>]>>`, replacing the Rc on registration, which is build-time work. Then clone the Rc here: `let listeners = Rc::clone(&node.geometry_listeners.borrow());` and iterate it. This avoids a Vec allocation for every node whose laid-out geometry changes.

### A213-03 - crates/incular-widgets/src/dx/motion/lint.rs:91 (§2, low)
Evidence:
```
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
```
Suggested fix: Collect once into a Vec and dedup in place: `let mut props: Vec<Prop> = motion.states[at + 1..].iter().filter(..).flat_map(..).collect(); props.sort_unstable(); props.dedup();`. Prop is Ord (BTreeSet requires it), so the sorted unique result is identical without the intermediate BTreeSet allocation.

### A213-02 - crates/incular-widgets/src/dx/motion/lint.rs:113 (§11, medium)
Evidence:
```
pub(crate) fn report(found: &[MotionLint]) {
    if !cfg!(debug_assertions) {
        return;
```
Suggested fix: Move the release gate in front of the lint pass so release builds never compute lints: change the signature to `pub(crate) fn report(motion: &Motion, child_keyed: bool)`, put `if !cfg!(debug_assertions) { return; }` first, then `for lint in lints(motion, child_keyed) { ... }`. Update the single caller at node.rs:1122 (`super::lint::report(&spec, key.is_some());`). Today `motion_widget` runs `lints(..)` on every `.motion(..)` build in all profiles and `report` discards the result in release.

### A216-02 - crates/incular-widgets/src/dx/motion/presence.rs:45 (§3, low)
Evidence:
```
let count = child.kind().structure().children.into_iter().count();
```
Suggested fix: Use `let count = child.kind().structure().children.into_iter().len();`. WidgetChildrenIter implements ExactSizeIterator, so this is O(1) instead of walking the children.

### A216-01 - crates/incular-widgets/src/dx/motion/presence.rs:60 (§11, high)
Evidence:
```
} else if spec.enter.is_some() {
Some(StaggerScope::empty())
```
Suggested fix: Reuse the previous empty scope instead of allocating two Rc cells on every build. Hoist `let previous = node.child_scope.borrow().clone();` above the `let scope = ...` statement and replace the `else if` arm with `} else if spec.enter.is_some() { match previous { Some(scope) if scope.stagger.is_none() => Some(scope), _ => Some(StaggerScope::empty()) } }`. An empty scope carries no per-build state: stagger_delay returns early when stagger is None.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
