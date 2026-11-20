# S1 brief s1-lower: public panic contracts

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

### A006-02 - crates/incular-animation/src/curve2d.rs:203
```
        assert!(
            (0.0..=1.0).contains(&tension),
```
Audit note: Add a fallible constructor paired with the panicking one, following the CatmullRomCurve pattern: `pub fn try_with_handles(control_points: &[Offset], tension: f32, start_handle: Option<Offset>, end_handle: Option<Offset>) -> Result<Self, CatmullRomSplineError>` returning a typed error for `tension` out of range and for fewer than four control points, and make `with_handles` (and `new`) call it and `expect`/panic with the same message, keeping the `# Panics` docs. Keep the `new` panic contract documented. Note: the tension and point-count checks are reachable from app-author input, and `CatmullRomSpline` currently has no `try_*` form.

### A010-03 - crates/incular-animation/src/scheduler.rs:101
```
pub fn set_time_dilation(dilation: f64) {
    assert!(
        dilation.is_finite() && dilation > 0.0,
```
Audit note: Add a fallible form and keep the panic only as its documented wrapper: `pub fn try_set_time_dilation(dilation: f64) -> Result<(), InvalidTimeDilation>` returning `Err(InvalidTimeDilation(dilation))` when `!(dilation.is_finite() && dilation > 0.0)`, with a `#[derive(Clone, Copy, Debug, PartialEq)] pub struct InvalidTimeDilation(pub f64);` plus Display/Error impls. Then `pub fn set_time_dilation(dilation: f64) { try_set_time_dilation(dilation).expect("time dilation was validated by the documented contract") }` with the existing `# Panics` doc kept. Re-export the new error from the crate root and mention the pair in the CHANGELOG.

### A062-03 - crates/incular-gestures/src/force_press.rs:106
```
assert!(peak > start, "the peak pressure exceeds the start pressure");
```
Audit note: Public builder panics on app-author input without the paired form the repo requires. Add `#[must_use] pub fn maybe_pressures(mut self, start: f32, peak: f32) -> Option<Self>` (or a typed error enum) that returns None when `!(peak > start)`, and make `pressures` a documented wrapper that calls `.expect("peak pressure exceeds start pressure")` on it.

### A120-05 - crates/incular-rendering/src/display_list.rs:670
```
match self.saves.pop().expect("unbalanced canvas restore") {
    SaveKind::Transform => self.list.push(PaintCommand::PopTransform),
    SaveKind::Clip => self.list.push(PaintCommand::PopClip),
```
Audit note: `Canvas` is re-exported by the `incular` facade, so an app author's unmatched `restore()` panics. Either return a structured error: `pub fn try_restore(&mut self) -> Result<(), UnbalancedCanvas>` (with `UnbalancedCanvas` a small error type in this crate) plus `restore` documented with `# Panics` as the contract wrapper; or document `# Panics` on `restore` and add the `try_restore` pair per CONTRIBUTING.md. Replace the `expect` with `let Some(kind) = self.saves.pop() else { return Err(UnbalancedCanvas) };` in the try form.

### A120-06 - crates/incular-rendering/src/display_list.rs:677
```
assert!(self.saves.is_empty(), "unbalanced display list");
```
Audit note: Public `Canvas::finish` panics on app-author input (a save without a matching restore). Add a `# Panics` section to its doc comment and a `try_finish(self) -> Result<DisplayList, UnbalancedCanvas>` form that returns the error, with `finish` delegating to it and calling `expect` on the error; or return the error only from `try_finish`.

### A140-04 - crates/incular-runtime/src/reactive.rs:302
```
assert!(
    !self.evaluating.replace(true),
    "reactive memo dependency cycle"
```
Audit note: A memo whose compute reads itself is an application-author mistake, so the panic is reachable from app input and is undocumented on the public `Memo::get`/`with`/`get_untracked`/`with_untracked`. Add a `# Panics` section to those methods documenting the cycle panic, and add a fallible form that returns an error, e.g. `pub fn try_get(&self) -> Result<T, MemoCycleError>` with `pub struct MemoCycleError;` (or make recompute return Result and thread it through ensure_fresh). Repo policy requires the documented panic to be paired with a try_* form.

### U025-01 - crates/incular-core/src/geometry.rs:24
```
assert!(
            width.is_finite() && height.is_finite() && width >= 0.0 && height >= 0.0,
```
Audit note: Add `maybe_new(width: f32, height: f32) -> Option<Self>` returning `None` for non-finite or negative inputs, and have `new` delegate to it. Document `new` with a `# Panics` section stating the contract ("Panics if either extent is non-finite or negative; use `maybe_new` for untrusted input"). Keep the existing message in the panic text. Use `maybe_new` internally where geometry derives sizes from app-supplied values (for example `intersection`, which can receive app-built rects).

### U026-01 - crates/incular-core/src/reactivity.rs:195
```
    assert!(
        READ_ONLY.with(Cell::get) == 0,
        "reactive mutation during a pure build or memo evaluation"
```
Audit note: Add a fallible form beside the panic contract: `pub fn try_assert_mutation_allowed() -> Result<(), ReactiveMutationError>` (new `#[derive(Debug, thiserror::Error)]` enum with a `MutationDuringPureEvaluation` variant) that returns `Err` when `READ_ONLY.with(Cell::get) != 0`. Keep `assert_mutation_allowed` as a thin wrapper that calls it and `expect`s, so the documented panic contract has its paired `try_*` form per AGENTS.md repo facts.
