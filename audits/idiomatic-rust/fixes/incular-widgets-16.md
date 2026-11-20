# Fix brief incular-widgets-16

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/hero.rs`, `crates/incular-widgets/src/painting_effects/shape_borders.rs`. Other agents are editing other files at
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
### A227-01 - crates/incular-widgets/src/hero.rs:579 (§2, high)
Evidence:
```
self.manifest.to_route.0.heroes().values().find_map(|hero| {
```
Suggested fix: Do not build a HashMap per flight per frame. Add a non-allocating method on RouteHeroesInner, e.g. `fn enabled_member(self: &Rc<Self>, entry: &Rc<HeroEntry>) -> bool` that runs the same retain (drop dead or foreign weaks) and then `heroes.iter().filter_map(Weak::upgrade).any(|hero| Rc::ptr_eq(&hero, entry) && hero.enabled.get())`. Then in `tick` use: `let origin = if self.aborted || !self.manifest.to_route.0.enabled_member(&self.manifest.to_hero) { None } else { self.manifest.to_hero.location().map(|rect| rect.origin) };`. Keep the retain side effect.

### A227-03 - crates/incular-widgets/src/hero.rs:718 (§12, medium)
Evidence:
```
let mut flights = self.0.flights.borrow_mut();
for (tag, from_hero) in from_heroes {
flights.insert(tag, Flight::start(manifest, from, to));
```
Suggested fix: `Flight::start` and `divert` call `manifest.tween`, which runs the user-supplied create_rect_tween closure, while `flights` is mutably borrowed. A closure that re-enters HeroFlights panics on the RefCell. Compute the tween (and the Flight) before taking the borrow, or call the tween outside the borrow by computing the `HeroRectTween` values in the loop first and inserting under a short `borrow_mut`. Keep the same order of side effects.

### A227-06 - crates/incular-widgets/src/hero.rs:765 (§3, low)
Evidence:
```
let ready: Vec<PendingTransition> = {
let mut pending = self.0.pending.borrow_mut();
let mut ready = Vec::new();
```
Suggested fix: Replace the index loop with `let ready: Vec<PendingTransition> = self.0.pending.borrow_mut().extract_if(.., |transition| { transition.waited += 1; transition.to.generation() != transition.generation || transition.waited > 2 }).collect();` (Vec::extract_if is stable). Removes the manual index and the O(n) remove calls, same order.

### A227-02 - crates/incular-widgets/src/hero.rs:802 (§12, medium)
Evidence:
```
let mut flights = self.0.flights.borrow_mut();
for flight in flights.values_mut() {
flight.manifest.shuttle(context),
```
Suggested fix: The `flights` RefCell is mutably borrowed while the user-supplied flight_shuttle_builder runs (via `shuttle(context)`) and while user create_rect_tween callbacks run (via `tick`). A builder that reaches HeroFlights::tags() or any other method that borrows `flights` would panic. Clone the cheap Rc fields needed (Manifest is Rc-based) and drop the borrow before calling `shuttle`, e.g. collect `(manifest.clone(), rect, opacity)` into a pre-sized buffer under the borrow, then build the widgets after `drop(flights)`.

### A227-05 - crates/incular-widgets/src/hero.rs:803 (§2, medium)
Evidence:
```
let mut shuttles = Vec::new();
for flight in flights.values_mut() {
flight.tick();
```
Suggested fix: The length is known: `let mut shuttles = Vec::with_capacity(flights.len());` (after the borrow, before the loop). This runs every frame while a flight is active.

### A245-02 - crates/incular-widgets/src/painting_effects/shape_borders.rs:66 (§11, high)
Evidence:
```
canvas.stroke_path(
    Arc::new(path),
    side.color,
```
Suggested fix: Every shape paint builds a fresh Path (Vec-backed builder) and boxes it in an Arc (also rounded_rect_path/ellipse_path/bevel_path callers at lines 99-124, 150-175, 221-246, 293-311, 339-353, 381-404, 417-430, 435-463, 932-985). This is per element per frame. Fix at the renderer boundary: let Canvas take a borrowed path or a stroke-rounded-rect/ellipse primitive so the shape borders do not allocate per paint. If the API must stay, cache the Arc<Path> in the decoration layer keyed by (rect, side, radii) and invalidate on change.

### A245-04 - crates/incular-widgets/src/painting_effects/shape_borders.rs:696 (§18, low)
Evidence:
```
#[allow(clippy::cast_possible_truncation)]
fn generate(&self, rect: Rect) -> Path {
```
Suggested fix: Replace with `#[expect(clippy::cast_possible_truncation, reason = "path coordinates are f32; f64 math is narrowed on output")]`. Same for `#[allow(clippy::cast_possible_truncation, clippy::float_cmp)]` at line 833 (draw_points) and `#[allow(clippy::float_cmp)]` at line 884 (get_angle), each with a reason string.

### A246-03 - crates/incular-widgets/src/painting_effects/shape_borders.rs:696 (§18, low)
Evidence:
```
#[allow(clippy::cast_possible_truncation)]
fn generate(&self, rect: Rect) -> Path {
```
Suggested fix: Replace with `#[expect(clippy::cast_possible_truncation, reason = "f64 path points narrow to f32 Offset coordinates by design")]`. Same change applies to the other allow attributes in this range at line 833 (`#[allow(clippy::cast_possible_truncation, clippy::float_cmp)]` on draw_points) and line 884 (`#[allow(clippy::float_cmp)]` on get_angle), each with its own reason.

### A245-01 - crates/incular-widgets/src/painting_effects/shape_borders.rs:709 (§2, high)
Evidence:
```
let mut points = Vec::new();
let max_diameter = 2.0
    * self.generate_points(
```
Suggested fix: Size the buffer once from the known point count instead of growing it per paint: `let mut points = Vec::with_capacity(self.points.ceil() as usize + 1);` (StarGenerator::generate runs for every outer_path/inner_path/paint of a StarBorder). Better still, hoist the Vec into a reused scratch buffer owned by the caller so steady-state paint does not allocate; the star's point count is bounded by its builder inputs.

### A246-02 - crates/incular-widgets/src/painting_effects/shape_borders.rs:709 (§2, medium)
Evidence:
```
let mut points = Vec::new();
let max_diameter = 2.0
```
Suggested fix: Size the buffer from the known side count: `let mut points = Vec::with_capacity(self.points.ceil() as usize + 1);` (generate_points pushes ceil(self.points) entries, so no regrowth). The function is already under #[allow(clippy::cast_possible_truncation)].


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
