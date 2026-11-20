# S1 brief s1-widgets: public panic contracts

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework, pre-1.0). Read
AGENTS.md "Errors vs panics" and CONTRIBUTING.md's error and panic policy first.
Policy: mistakes by application authors return structured errors or diagnostics;
broken framework invariants may `expect("<invariant>")`; public APIs panic only
as a documented contract, paired with a `maybe_*`/`try_*` form.

Each finding below is a public function that panics. Decide per site:

1. **Value validation on app-author input** (a constructor or setter that
   asserts its arguments are finite, positive, in range and so on):
   - add a `try_*` form returning `Result<Self, E>` (or `Result<(), E>`) with a
     small typed error, `#[non_exhaustive]` enum with Display + std::error::Error;
     reuse an existing error type in the crate if one fits;
   - keep the panicking form as a thin wrapper over `try_*` that `expect`s,
     with a `# Panics` section naming the condition and pointing to `try_*`;
   - keep behavior identical for valid input; keep Flutter-parity semantics.
   - A builder setter on a widget struct can take the same route
     (`try_points(self, ..) -> Result<Self, E>`).
2. **Programming-contract panics** (unbalanced save/restore, a dependency cycle,
   mutation during a pure build, a missing child id in a layout delegate,
   inconsistent flag combinations): keep the panic, add a `# Panics` doc
   section that states the contract. Add a `try_*`/`maybe_*` form only where a
   caller can reasonably recover (e.g. a fallible restore/finish), not for
   cycles or build-phase violations.
3. If the panic is in a private fn, document it on the public entry point that
   reaches it.

Rules: edit only the files listed in the findings plus the crate's lib.rs
re-exports for new error types; `rustdoc::broken_intra_doc_links` is denied so
links must resolve; no `#[cfg(test)]` code under src/; you may add tests in
the crate's existing `tests/` files for new `try_*` forms. Add ONE CHANGELOG.md
entry under `## Unreleased` -> `### Added` (create that heading above
`### Changed` if missing) listing the new `try_*` forms. Edit CHANGELOG.md with
a single Edit call at the very end, since another agent edits it too.
Do NOT run cargo (the orchestrator builds); do not commit.

Reply: one line per finding `<id>: <try_ form added | documented only> - <api>`,
then the new public items.

## Findings

### A188-01 - crates/incular-widgets/src/advanced_scrolling/draggable.rs:143
```
assert!(min_size.is_finite() && max_size.is_finite() && initial_size.is_finite());
assert!(min_size >= 0.0 && min_size <= max_size && max_size <= 1.0);
assert!(initial_size >= min_size && initial_size <= max_size);
```
Audit note: Add a fallible constructor, e.g. `pub fn try_new(min, max, initial, close) -> Result<Self, DraggableExtentError>` that returns an error for the three predicates. Keep `new` only as a documented contract: `/// # Panics` plus `.expect(...)` over `try_new`. Change `DraggableScrollableSheet::extents` (line 591, `let _ = DraggableSheetExtent::new(...)`) to validate through `try_new` and surface the error (or add `try_extents` alongside a documented-panic `extents`). Reached from app-author configuration via `extents()` and `mount()`.

### A191-01 - crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:224
```
assert!(horizontal.axis() == Axis::Horizontal);
        assert!(vertical.axis() == Axis::Vertical);
```
Audit note: Public API panics on app-author input without a documented # Panics contract or a maybe_/try_ form. Return a typed error instead, e.g. `pub fn set_axis_directions(&mut self, horizontal: AxisDirection, vertical: AxisDirection) -> Result<(), AxisDirectionError>` with `if horizontal.axis() != Axis::Horizontal { return Err(AxisDirectionError::HorizontalNotHorizontal(horizontal)); }` and the same check for vertical, storing the fields only after both checks pass. Alternatively keep the panic, add a `# Panics` rustdoc section and provide `try_set_axis_directions` returning Result.

### A192-05 - crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:626
```
assert!(row_estimate.is_finite() && row_estimate > 0.0);
assert!(column_estimate.is_finite() && column_estimate > 0.0);
(also lines 687-688 set_axis_directions asserts and line 700 set_cache_extent assert)
```
Audit note: These panic on values an app author passes to public constructors/setters, and the rustdoc has no # Panics section. Either document the contract (`# Panics` in the doc comments of new, set_axis_directions and set_cache_extent) and add maybe_/try_ forms returning Result (e.g. try_new, try_set_cache_extent) with a typed error, per CONTRIBUTING.md; or return the error from the public API directly. Converting to Result changes the signature, so note it as breaking in the CHANGELOG.

### A193-01 - crates/incular-widgets/src/advanced_scrolling/wheel.rs:315
```
assert!(diameter_ratio.is_finite() && diameter_ratio > 0.0);
assert!(perspective.is_finite() && perspective > 0.0 && perspective <= 0.01);
assert!(off_axis_fraction.is_finite());
```
Audit note: Panic reachable from app-author input with no # Panics doc and no try_/maybe_ form. Add `pub fn try_new(diameter_ratio: f32, perspective: f32, off_axis_fraction: f32, squeeze: f32) -> Result<Self, InvalidWheelProjection>` returning Err for the four checks, and keep `new` as a documented panicking form (`# Panics` section) that calls `try_new(..).expect("...")` or unwraps the Result with a documented contract.

### A193-02 - crates/incular-widgets/src/advanced_scrolling/wheel.rs:560
```
assert!(item_extent.is_finite() && item_extent > 0.0);
Self {
    controller,
```
Audit note: Panic on app-author input (item_extent of 0, NaN or negative) in the public constructor with no # Panics doc. Add `try_new(controller, item_extent, delegate) -> Result<Self, InvalidItemExtent>` (and a matching `try_` form for FixedExtentScrollController::with_item_extent at line 1012, which has the same assert), documenting `new` as the panicking contract.

### A193-03 - crates/incular-widgets/src/advanced_scrolling/wheel.rs:620
```
assert!(magnification.is_finite() && magnification > 0.0);
assert!((0.0..=1.0).contains(&opacity));
assert!(!outside_viewport || clip_behavior == Clip::None);
```
Audit note: Three public setters (set_magnifier, set_over_under_center_opacity, set_rendering_policy) panic on app-author input with no # Panics doc and no try_/maybe_ form. Provide `try_set_magnifier`, `try_set_over_under_center_opacity` and `try_set_rendering_policy` returning Result with a typed error, and document the panicking originals with a # Panics section (or have them delegate with expect).

### A205-02 - crates/incular-widgets/src/custom_layout.rs:285
```
            .get(&id)
            .unwrap_or_else(|| panic!("CustomMultiChildLayout has no child with id {id:?}"))
```
Audit note: Public panicking methods here have no maybe_/try_ pair, which CONTRIBUTING and AGENTS require. Add `pub fn try_layout_child(&mut self, id: impl Into<LayoutKey<'_>>, constraints: Constraints) -> Result<Size, MissingLayoutChild>` and `try_position_child(..) -> Result<(), MissingLayoutChild>` returning a typed error (`MissingLayoutChild` holding the id), then keep layout_child/position_child as documented-panic wrappers over the try_ form. Likewise FlowPaintingContext::paint_child (358-360, out-of-range index and double paint) needs a try_paint_child returning an error for those two cases. Note: the panics are documented under # Panics, so this is a missing pair, not an undocumented panic.

### A209-03 - crates/incular-widgets/src/drag_boundary.rs:71
```
assert!(
    right - dragged.size.width >= boundary.origin.x
        && bottom - dragged.size.height >= boundary.origin.y,
```
Audit note: Repo rule: a public panic must be a documented contract paired with a maybe_/try_ form, and this one has no pair. Add an inherent method on RectDragBoundary: pub fn try_nearest_position_within_boundary(&self, dragged: Rect) -> Option<Rect>, returning None when dragged is larger than the boundary (the check at lines 71-76 becomes the None branch) and Some(clamped rect) otherwise. Then make the DragBoundaryDelegate::nearest_position_within_boundary impl call it and use expect with the existing message text as the documented panic. Keep the # Panics doc on the trait method. Behavior of the panicking path is unchanged.

### A237-02 - crates/incular-widgets/src/magnifier.rs:188
```
pub fn magnification_scale(mut self, scale: f32) -> Self {
    assert!(
        scale != 0.0,
```
Audit note: Repo policy forbids panics reachable from app-author input unless documented as a contract with a try_/maybe_ form. Either return a structured result via `pub fn try_magnification_scale(self, scale: f32) -> Result<Self, MagnifierScaleError>` (with a typed error enum) and keep `magnification_scale` as a documented `# Panics` wrapper, or store the value and report a diagnostic instead of asserting. Add a `# Panics` section if the assert is kept.

### A245-03 - crates/incular-widgets/src/painting_effects/shape_borders.rs:519
```
pub fn points(mut self, points: f32) -> Self {
    assert!(points >= 2.0, "a star has at least two points");
```
Audit note: Either return a Result (e.g. `pub fn try_points(self, points: f32) -> Result<Self, StarBorderError>`) and keep `points` infallible only with a `# Panics` doc section, or clamp/validate and report through the framework diagnostic path. Same for inner_radius_ratio (line 527), point_rounding (535-536), valley_rounding (544-545) and squash (561): these asserts take app-author values and their doc comments do not state a panic contract or offer a try_ form, which CONTRIBUTING.md requires.

### A267-01 - crates/incular-widgets/src/scrolling/sliver_descriptors.rs:2023
```
fn assert_flags(&self) {
        assert!(
            !self.maintain_interactivity || self.maintain_size,
```
Audit note: Replace the four assert! calls with a fallible check returning a typed error, e.g. `fn check_flags(&self) -> Result<(), SliverVisibilityFlagError>` (a #[non_exhaustive] enum with one variant per dependency: InteractivityNeedsSize, SemanticsNeedsSize, SizeNeedsAnimation, AnimationNeedsState). Add `try_build` / `try_create_render_sliver` that return the error, and keep `build` / `create_render_sliver` panicking only as a documented contract (`# Panics` section on the struct and both methods) that points to the try_ form, per CONTRIBUTING's panic policy. Called from build at line 2065 and create_render_sliver at line 2081.

### A332-01 - crates/incular-material/src/theme/state.rs:240
```
.map_or_else(
    || panic!("StateProperty::Map cannot resolve an empty map"),
    |(_, value)| value.clone(),
```
Audit note: Panic is reachable from app-author input: `StateProperty::from_map([])` and the public `StateProperty::Map(vec![])` variant both produce an empty map. Per the repo policy, keep the panic only as a documented contract with a fallible twin. Add a `# Panics` section to `resolve` that names `maybe_resolve`, and add `pub fn maybe_resolve(&self, states: WidgetStates) -> Option<T>` that returns `None` for an empty `Map` (for `All` and `Resolver` it returns `Some`). Then `resolve` becomes `self.maybe_resolve(states).expect("StateProperty::Map must not be empty; use maybe_resolve for user-built maps")`.
