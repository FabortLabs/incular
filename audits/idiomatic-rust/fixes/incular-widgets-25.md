# Fix brief incular-widgets-25

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/examples/mem_probe.rs`, `crates/incular-widgets/src/advanced_scrolling/raw_scrollbar.rs`, `crates/incular-widgets/src/advanced_slivers/keep_alive.rs`, `crates/incular-widgets/src/app_shell/view.rs`, `crates/incular-widgets/src/async_builders.rs`, `crates/incular-widgets/src/context_menu.rs`, `crates/incular-widgets/src/display_feature_sub_screen.rs`, `crates/incular-widgets/src/dx/motion/cond.rs`, `crates/incular-widgets/src/dx/motion/dnd.rs`, `crates/incular-widgets/src/dx/motion/shared.rs`, `crates/incular-widgets/src/environment.rs`. Other agents are editing other files at
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
### U123-01 - crates/incular-widgets/examples/mem_probe.rs:14 (§16, high)
Evidence:
```
unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        unsafe {
```
Suggested fix: Add a `// SAFETY:` comment above the `unsafe impl` (line 14) explaining that every call forwards to `std::alloc::System` with the caller's layout and pointer unchanged and only adds a relaxed counter update. Add a second `// SAFETY:` comment above each `unsafe {` block in `alloc` (line 16) and `dealloc` (line 25) stating the same forwarding contract.

### U123-02 - crates/incular-widgets/examples/mem_probe.rs:34 (§18, low)
Evidence:
```
#[allow(dead_code)]
fn live_mb() -> usize {
    live_kb() / 1024
```
Suggested fix: `live_mb` is never called (main only uses `live_kb`). Delete lines 34-37 (the attribute and the function). If it is kept, use `#[expect(dead_code, reason = "diagnostic helper for MB output")]` instead of `#[allow(dead_code)]`.

### A190-01 - crates/incular-widgets/src/advanced_scrolling/raw_scrollbar.rs:539 (§11, low)
Evidence:
```
let content_extent = self.controller.content_extent().max(0.0);
let viewport_extent = self.controller.viewport_extent().max(0.0);
let min_scroll_extent = self.controller.min_offset().min(0.0);
```
Suggested fix: Take one snapshot at the top of layout_geometry and reuse it: `let metrics = self.controller.metrics();`, then read min/max/pixels/axis_direction from `metrics` (lines 541-542, 580-583, 592) instead of re-querying the controller. Change `is_reversed` (line 224-227) to take the `axis_direction` (or `&ScrollController` replaced by the already-fetched metrics) so `controller.metrics()` is not called a second time at line 596. Verify `metrics` exposes the content/viewport extents or keep those two getters; only remove the duplicate `metrics()` and `offset()` calls.

### A195-07 - crates/incular-widgets/src/advanced_slivers/keep_alive.rs:147 (§10, low)
Evidence:
```
if state.clients.contains_key(&id) {
    return false;
```
Suggested fix: contains_key followed later by clients.insert(id, ..) is a double hash lookup. Use the entry API at the insertion point (`Entry::Occupied` returns false, `Entry::Vacant` reserves the slot) or hoist the check next to the insert, so the lookup happens once.

### A202-01 - crates/incular-widgets/src/app_shell/view.rs:428 (§2, low)
Evidence:
```
.filter_map(Weak::upgrade)
.collect::<Vec<_>>()
listener(event.clone());
```
Suggested fix: ViewController::notify (and ViewAnchorController::notify at lines 845-851 with the same pattern) allocates a fresh Vec of upgraded listeners on every published event, and clones `event` once per listener. Keep the upgraded list in a reusable scratch buffer owned by ViewState (e.g. `scratch: Vec<Rc<dyn Fn(ViewEvent)>>`, filled with `clear()` + `extend`, then drained after the borrow is released), and move `event` into the last listener instead of cloning for every listener. Keeps the borrow released before callbacks run.

### A203-02 - crates/incular-widgets/src/async_builders.rs:309 (§2, medium)
Evidence:
```
*snapshot = Some(current.clone());
        current
    }
```
Suggested fix: Store the snapshot as `RefCell<Option<Rc<AsyncSnapshot<T, E>>>>`. `FutureBuilderState::snapshot` then returns `Rc<AsyncSnapshot<T, E>>`, so each build clones an Rc instead of the data `T`/`E`. The builder is called with `&snapshot`. The same pattern exists in `StreamBuilderState::build` (line 603, `state.summary.borrow().clone().expect("summary")`), where `summary` should become `RefCell<Option<Rc<B::Summary>>>` and `fold` wraps the step result in `Rc::new`. Avoid holding the RefCell borrow across the user build callback.

### A204-02 - crates/incular-widgets/src/context_menu.rs:296 (§11, medium)
Evidence:
```
let menu = state.menu();
            let mut portal = OverlayPortal::new(SizedBox::shrink())
```
Suggested fix: Store the open menu as `RefCell<Option<Rc<TextContextMenu>>>` in TextContextMenuState (show() wraps it in Rc::new once). Then in the build closure take `let menu = state.menu.borrow().clone();` (Rc clone, no Vec<ContextMenuButtonItem> or label String reallocation) and move the Rc into overlay_child_layout_builder(move |context, _| builder(context, &menu)). Make the public `menu()` return Option<Rc<TextContextMenu>> or keep it cloning only for external callers.

### A209-02 - crates/incular-widgets/src/display_feature_sub_screen.rs:89 (§2, medium)
Evidence:
```
let sub_screens =
    Self::sub_screens_in_bounds(wanted_bounds, &Self::avoid_bounds(&features));
```
Suggested fix: The Vec returned by avoid_bounds is only read as a slice, so build allocates it for nothing. Make the splitting core take an iterator: fn split_sub_screens(wanted_bounds: Rect, avoid: impl IntoIterator<Item = Rect>) -> Vec<Rect>, keep the public sub_screens_in_bounds(wanted, &[Rect]) as a thin wrapper that calls it with avoid.iter().copied(), and have build call split_sub_screens(wanted_bounds, features.iter().filter(|f| divides_view(f)).map(|f| f.bounds)) with the avoid_bounds predicate moved to a private fn divides_view(feature: &DisplayFeature) -> bool.

### A211-01 - crates/incular-widgets/src/dx/motion/cond.rs:179 (§2, medium)
Evidence:
```
pub(crate) struct GroupScopes(pub(crate) Rc<Vec<Rc<GroupState>>>);
...
let mut scopes = (*self.0).clone();
scopes.push(state);
```
Suggested fix: Make the shared list an `Rc<[Rc<GroupState>]>` and build it in one allocation: `pub(crate) struct GroupScopes(pub(crate) Rc<[Rc<GroupState>]>);` and `pub(crate) fn pushed(&self, state: Rc<GroupState>) -> Self { Self(self.0.iter().cloned().chain(std::iter::once(state)).collect()) }`. `nearest` keeps working on the slice. Check any other use of `GroupScopes.0` that expects a `Vec`, and the `Default` derive still compiles.

### A212-01 - crates/incular-widgets/src/dx/motion/dnd.rs:108 (§2, high)
Evidence:
```
let targets = shared.targets.borrow().clone();
```
Suggested fix: Change `Shared::targets` to `RefCell<Rc<[Rc<Target<T>>]>>`. Rebuild the Rc in `bind` (after the push) and when `refresh` drops a dead target from the retain (both are rare, build or teardown only). Then clone the Rc here: `let targets = Rc::clone(&shared.targets.borrow());`. Every pointer move through `refresh` then does no heap allocation.

### A217-08 - crates/incular-widgets/src/dx/motion/shared.rs:351 (§11, medium)
Evidence:
```
} else if channel.holds() {
            let Some(key) = channel.key.borrow().clone() else {
```
Suggested fix: For a joined holder, geometry() does a registry lookup in holds() and then a second lookup and key clone to write entry.last. Do one lookup: `} else if let Some(key) = channel.key.borrow().as_ref() { REGISTRY.with(|registry| { if let Some(entry) = registry.borrow_mut().get_mut(key) && entry.holder.as_ref().is_some_and(|holder| is(holder, channel)) { entry.last = Some(now.visual_rect); } }); }`

### A220-02 - crates/incular-widgets/src/environment.rs:409 (§18, low)
Evidence:
```
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct OrientationBuilder {
```
Suggested fix: Introduce a named alias so the complexity lint no longer fires and the attribute can be removed:
type OrientationCallback = Rc<dyn for<'a> Fn(&BuildContext<'a>, Orientation) -> Widget>;
// then: builder: OrientationCallback,  and drop the #[allow]. If clippy still fires, use #[expect(clippy::type_complexity, reason = "...")] instead.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
