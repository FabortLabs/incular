# Fix brief incular-widgets-23

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/dx/motion/timeline.rs`, `crates/incular-widgets/src/dx/view_transition.rs`, `crates/incular-widgets/src/explicit_animations.rs`, `crates/incular-widgets/src/flutter_logo.rs`, `crates/incular-widgets/src/implicit_animations.rs`, `crates/incular-widgets/src/navigation/back_dispatch.rs`. Other agents are editing other files at
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
### A218-02 - crates/incular-widgets/src/dx/motion/timeline.rs:648 (§2, medium)
Evidence:
```
let wake: Rc<dyn Fn()> = {
    let dirty = self.live_dirty.clone();
    Rc::new(move || {
```
Suggested fix: bind() allocates a new Rc<dyn Fn()> wake closure (and a new subscriptions Vec, re-subscribing every source) on every call. Paint-tier effects make the node rebuild each frame, so bind can run per frame. Create the wake Rc once in Timeline::new (store `wake: Rc<dyn Fn()>` capturing a clone of live_dirty) and use `self.wake.clone()` in bind. Also skip the `written_props` recomputation when `Rc::ptr_eq` shows the spec is unchanged, and only re-subscribe when the Cond or Progress source actually changed.

### A218-03 - crates/incular-widgets/src/dx/motion/timeline.rs:724 (§3, low)
Evidence:
```
let moved = self.position.replace(head.progress()) != head.progress();
```
Suggested fix: Read the progress once: `let next = head.progress(); let moved = self.position.replace(next) != next;`

### A219-02 - crates/incular-widgets/src/dx/view_transition.rs:248 (§11, high)
Evidence:
```
output.extend_from(&transition.old);
...
output.extend_from(&transition.old);
```
Suggested fix: Every frame of a transition, extend_from clones each PaintCommand of the old scene into the output list (Arc bumps per command, plus Vec growth) even though the old scene never changes. Splice the shared old buffer by reference instead: add a DisplayList method (e.g. `push_shared(&mut self, other: &DisplayList)`) that records a single reference command such as `PaintCommand::Shared(Arc<Vec<PaintCommand>>)`, and have the display-list walker expand it, so the per-frame cost becomes one Arc bump. Both call sites (lines 248 and 280) use it. If that variant is judged too invasive for the renderer, record it as a known cost in the rendering notes rather than leaving it implicit.

### A219-01 - crates/incular-widgets/src/dx/view_transition.rs:267 (§11, high)
Evidence:
```
let mut circle = Path::builder();
circle.rounded_rect(
...
let hole = Path::rect_excluding(bounds, &circle.build());
```
Suggested fix: Per frame while a circular reveal runs, a PathBuilder is allocated, built into a temporary interned Path, and then rect_excluding clones its BezPath and interns a second Path under the INTERNED_PATHS mutex before Arc::new. Replace the builder and temporary with one call that writes the hole directly, e.g. add in incular-rendering `Path::rect_excluding_circle(outer: Rect, center: Offset, radius: f32) -> Self` (a single BezPath: outer rect plus the circle as four cubic arcs, same winding handling as rect_excluding) and here: `let hole = Path::rect_excluding_circle(bounds, center, radius);`. Keep the pushed `Arc::new(hole)` as the only per-frame allocation. Alternatively keep the circle path inside Active and reuse it, but the radius changes each frame so the geometry must be regenerated.

### A221-02 - crates/incular-widgets/src/explicit_animations.rs:49 (§2, low)
Evidence:
```
for controller in controllers {
    let on_change = on_change.clone();
    let id = controller.add_listener(move |_| on_change());
```
Suggested fix: Reserve the known length before the loop: `listening.reserve(controllers.len());` (after `let mut listening = self.listening.borrow_mut();`). Only runs when the controller set changes, not in steady state.

### A221-01 - crates/incular-widgets/src/explicit_animations.rs:106 (§2, high)
Evidence:
```
let controllers = animation.controllers();
let revision = state.revision.borrow().clone();
let weak: Weak<Cell<u64>> = Rc::downgrade(&revision);
```
Suggested fix: The controller set of `animation` is fixed for the life of the widget, so compute it once and keep it in `RebuildState` (e.g. a `controllers: Vec<AnimationController>` field filled in the `retained_builder` init closure from `animation.controllers()`). Change `ticking` to take `&[AnimationController]` and iterate `controllers.iter().cloned()` (Rc clones, no Vec). `ControllerListeners::listen` already takes a slice. Per-tick rebuilds then allocate no Vec. Note `Clocks::controllers` (line 288) also clones its Vec on every call, which this change removes from the rebuild path.

### A222-01 - crates/incular-widgets/src/flutter_logo.rs:245 (§11, medium)
Evidence:
```
fn polygon(points: &[(f32, f32)]) -> Arc<Path> {
    let mut builder = Path::builder();
    ...
    Arc::new(builder.build())
```
Suggested fix: The mark's geometry is independent of the box size (only the transform changes), so build the shapes once and reuse them. Hoist the four polygons and the triangle GradientStops into a cached value, e.g. a `static MARK: LazyLock<MarkGeometry>` (or a `OnceLock` if Path/GradientStops are not Sync-friendly for LazyLock), and have paint_mark clone the cached Arc<Path> handles and call canvas.fill_path with them. Also replace the per-call `GradientStops::new(vec![...])` at lines 269-278 with the cached value. Keep the coordinate values unchanged. Confirm Path and GradientStops are Send + Sync before using a static.

### A222-02 - crates/incular-widgets/src/flutter_logo.rs:303 (§18, low)
Evidence:
```
#[allow(clippy::approx_constant)]
canvas.save_transform(CoreTransform::from_coefficients([
    0.7071, -0.7071, 0.7071, 0.7071, -77.697, 98.057,
```
Suggested fix: Replace `#[allow(clippy::approx_constant)]` with `#[expect(clippy::approx_constant, reason = "Flutter's matrix writes 0.7071, not 1/sqrt(2)")]` if the lint still fires on the 0.7071 literal. If clippy no longer flags it (approx_constant needs more digits than 0.7071), delete the attribute instead, because an unfulfilled #[expect] is itself a warning.

### A228-01 - crates/incular-widgets/src/implicit_animations.rs:300 (§12, medium)
Evidence:
```
            && let Some(on_end) = state
                .config
                .borrow()
```
Suggested fix: Take the callback out before calling it, so the RefCell Ref is dropped first: `if was && !state.progress.is_animating() { let on_end = state.config.borrow().as_ref().and_then(|config| config.timing().on_end.clone()); if let Some(on_end) = on_end { on_end(); } }`. Apply the same restructuring to the listener closure at lines 428-436 (`state.controller.status() == AnimationStatus::Completed && let Some(on_end) = state.config.borrow()...`), which has the same pattern. In let-chains and if-let the scrutinee temporary (the Ref) lives through the body, so the user's on_end callback runs while config is borrowed.

### A228-02 - crates/incular-widgets/src/implicit_animations.rs:580 (§2, medium)
Evidence:
```
        self.decoration
            .clone()
            .or_else(|| self.color.map(|color| BoxDecoration::new().color(color)))
```
Suggested fix: Return a borrow instead of a clone, since the caller only reads it: change the signature to `fn effective_decoration(&self) -> Option<Cow<'_, BoxDecoration>>` with body `match (&self.decoration, self.color) { (Some(d), _) => Some(Cow::Borrowed(d)), (None, Some(c)) => Some(Cow::Owned(BoxDecoration::new().color(c))), (None, None) => None }`, and at the call site in for_each_tween (line 625) pass `self.effective_decoration().as_deref()`. This avoids cloning the whole BoxDecoration (which can own heap data such as gradients or shadows) each time a container's configuration is updated (for_each_tween runs only when the config Rc changes, not every animation tick).

### A238-02 - crates/incular-widgets/src/navigation/back_dispatch.rs:467 (§23, medium)
Evidence:
```
self.dispatch_back_inner(0, &mut HashSet::new(), None)
...
if !visited.insert(identity) {
```
Suggested fix: The visited set is created and inserted into on every back dispatch and every child recursion, and HashSet::insert allocates its table. Cycles are possible (attach_child does not reject ancestors), so keep cycle detection but store identities inline: change `visited: &mut HashSet<usize>` to `visited: &mut SmallVec<[usize; 8]>`, use `let mut visited = SmallVec::new();` in dispatch_back and dispatch_back_with_fallback, and replace the insert with `if visited.contains(&identity) { return ...; } visited.push(identity);`. Dispatcher depth is tiny, so the linear scan is cheaper than hashing. Drop the HashSet import if unused.

### A238-01 - crates/incular-widgets/src/navigation/back_dispatch.rs:507 (§23, medium)
Evidence:
```
let children = state
.children
.iter()
```
Suggested fix: Each back event (dispatch_back_inner) collects three fresh Vecs (children, handlers, guards) plus a Vec passed into dispatch_pop. Keep the snapshot semantics (callbacks may mutate the registries during dispatch) but snapshot into inline storage: add `smallvec` to crates/incular-widgets/Cargo.toml if absent, then use `SmallVec<[Rc<RefCell<DispatcherState>>; 4]>` for children, `SmallVec<[BackHandler; 4]>` for handlers and `SmallVec<[(PopCanPop, PopInvoked); 4]>` for guards (collect with `.collect::<SmallVec<_>>()`), and change dispatch_pop's `guards` parameter to the same SmallVec type. Typical counts then never touch the heap.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
