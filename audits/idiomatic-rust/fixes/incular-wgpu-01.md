# Fix brief incular-wgpu-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-wgpu/src/renderer/frame.rs`, `crates/incular-wgpu/src/renderer/effects.rs`. Other agents are editing other files at
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
### A179-03 - crates/incular-wgpu/src/renderer/frame.rs:102 (§2, medium)
Evidence:
```
let bind_group = self
    .create_composite_bind_group(target, "incular straight-alpha presentation bind group");
```
Suggested fix: A GPU bind group is created on every presented frame in `present_straight_alpha_target` (line 102) and likewise in `present_composition_target` (line 505). The target is cached in `self.presentation_target`, so cache the bind group next to it: store `Option<wgpu::BindGroup>` keyed by the target's generation (or by recreating it inside `ensure_presentation_target` when the target is rebuilt), and reuse it here. Only the `write_buffer` of the instance data needs to happen per frame. Apply the same to the destination composition target if its bind group depends only on the target view.

### A179-01 - crates/incular-wgpu/src/renderer/frame.rs:276 (§2, medium)
Evidence:
```
let mut active = active_start.to_vec();
for batch in segment {
    match batch {
```
Suggested fix: Keep one persistent `active_stencils` Vec per destination frame and advance it in place after the segment is encoded, instead of allocating a copy per segment. Change `stencils_after` to `fn advance_stencils(active: &mut Vec<DrawBatch>, segment: &[DrawBatch])` that runs the same push/pop loop on `active` (no `to_vec`), and at line 351 remove `let next_active_stencils = Self::stencils_after(&active_stencils, painted);`. At line 378 replace `active_stencils = next_active_stencils;` with `Self::advance_stencils(&mut active_stencils, painted);`. `segment` (line 350) is last used at line 372, so the shared borrow has ended by then. Line 322 `let mut active_stencils = Vec::new();` can then be reused across frames via a renderer scratch field, if desired.

### A179-04 - crates/incular-wgpu/src/renderer/frame.rs:305 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
pub(super) fn render_destination_batches(
```
Suggested fix: Replace with `#[expect(clippy::too_many_arguments, reason = "destination pass threads the frame's target, clear and label state explicitly")]`. The same change applies to the `#[allow(clippy::too_many_arguments)]` at line 484 on `present_composition_target`. Both lints fire (8 and 9 inputs including `self`), so `expect` will be fulfilled.

### A179-02 - crates/incular-wgpu/src/renderer/frame.rs:323 (§2, medium)
Evidence:
```
let plans = self.plan_backdrops(batches, scale, width, height, clear);
let mut next_plan = 0_usize;
```
Suggested fix: `plan_backdrops` (renderer/backdrop_cache.rs:52) returns a fresh `Vec<BackdropPlan>` and builds three local `HashMap`/`Vec` structures on every call whenever the frame has a backdrop. Change it to take a caller-owned buffer, `fn plan_backdrops(&self, out: &mut Vec<BackdropPlan>, batches, scale, width, height, clear)`, that clears and refills `out`, and keep that Vec plus the signature/capture maps on the renderer as scratch state reused across frames (clear, don't reallocate). Here: `let mut plans = std::mem::take(&mut self.backdrop_plan_scratch); self.plan_backdrops(&mut plans, ...);` and restore it at the end of the function.

### A179-05 - crates/incular-wgpu/src/renderer/frame.rs:606 (§11, low)
Evidence:
```
let rectangles = batches
    .iter()
    .filter_map(|batch| match batch {
```
Suggested fix: `render_composited` walks the full `batches` list about ten times per frame (lines 606, 615, 624, 633, 643, 654, 665, 722, 728, 741, plus `commands_have_destination_blend` at line 584 over the commands). Compute all tallies in one pass: `let mut tally = FrameTally::default(); for batch in &batches { tally.record(batch); }` where `record` updates rectangles, glyphs, images, rounded, paths, stencil_masks, composites, needs_stencil, has_destination_blend and reads_destination_in_blend with the same clip filters as today. Equivalent output, one traversal. Better still, count while lowering in `lower_commands`, which already visits each batch.

### A177-03 - crates/incular-wgpu/src/renderer/effects.rs:474 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
pub(super) fn render_blur_from_source(
```
Suggested fix: Replace with `#[expect(clippy::too_many_arguments, reason = "blur inputs mirror the layer and effect cache key")]` on this function and on the identical attribute at line 537 (`render_blur`). Both lints fire (self plus seven parameters), so the expectation is fulfilled.

### A178-03 - crates/incular-wgpu/src/renderer/effects.rs:732 (§2, low)
Evidence:
```
return Ok((final_target, vec![horizontal]));
...
Ok((final_target, vec![downsample, horizontal, vertical]))
```
Suggested fix: Return the temporaries without a heap Vec on every blur re-render: use smallvec::SmallVec<[OffscreenTarget; 3]> (smallvec is already shared in this crate per recent commits) or a fixed [Option<OffscreenTarget>; 3] with a count. Callers at line 611 iterate the result once.

### A177-02 - crates/incular-wgpu/src/renderer/effects.rs:793 (§23, medium)
Evidence:
```
Ok((final_target, vec![downsample, horizontal, vertical]))
```
Suggested fix: Return the three temporaries in a SmallVec: change the return type at line 685 to `Result<(OffscreenTarget, smallvec::SmallVec<[OffscreenTarget; 3]>), RendererError>`, use `smallvec![downsample, horizontal, vertical]` here and `smallvec![horizontal]` at line 732. This runs on every blur re-render (source changed), so animated blurs allocate a Vec per frame today.

### A178-02 - crates/incular-wgpu/src/renderer/effects.rs:826 (§11, medium)
Evidence:
```
let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
 label: Some("incular color matrix pass bind group"),
 layout: &self.color_matrix_bind_group_layout,
```
Suggested fix: Same approach as record_effect_pass: reuse a cached bind group keyed by the source view revision (the params buffer and sampler are already persistent), rebuilding only when the source view changes or the device generation changes.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
