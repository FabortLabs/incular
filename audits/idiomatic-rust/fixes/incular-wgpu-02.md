# Fix brief incular-wgpu-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-wgpu/src/renderer/rendering.rs`, `crates/incular-wgpu/src/renderer/backdrop.rs`, `crates/incular-wgpu/src/renderer/lowering.rs`. Other agents are editing other files at
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
### A182-02 - crates/incular-wgpu/src/renderer/rendering.rs:145 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
pub(super) fn encode_batches(
```
Suggested fix: #[expect(clippy::too_many_arguments, reason = "encode_batches takes the full frame target and batch state; bundling them would add a per-frame struct without simplifying callers")]

### A182-05 - crates/incular-wgpu/src/renderer/rendering.rs:496 (§other, low)
Evidence:
```
DrawBatch::Offscreen {
    layer, instance, ..
} => {
```
Suggested fix: Drop the unused `instance` binding from the Offscreen, Filtered, Shadow and Blend arms (patterns at lines 496, 526, 556, 591) and delete the `let _ = instance;` statements at lines 522, 552, 587 and 637, 679. Use `..` in the patterns instead.

### A182-04 - crates/incular-wgpu/src/renderer/rendering.rs:561 (§11, low)
Evidence:
```
if let Some(entry) = self.effect_cache.get_mut(layer) {
    entry.last_used_frame = self.counters.frames;
} else if let Some(entry) = self.offscreen_cache.get_mut(layer) {
```
Suggested fix: Resolve the layer once per Shadow batch into a single enum/reference (e.g. `enum LayerEntry<'a> { Effect(&mut Entry), Offscreen(&mut Entry) }` or a lookup that returns which cache hit), then reuse that result for the timestamp update, the bind group and the continue check at lines 569-575 instead of up to four hash lookups of the same key.

### A182-03 - crates/incular-wgpu/src/renderer/rendering.rs:675 (§other, low)
Evidence:
```
if !mode.requires_destination_read() {
    self.counters.blend_fixed_function_draws += 1;
}
```
Suggested fix: Delete the branch. Line 617 already `continue`s when !mode.requires_destination_read(), so this condition is always false and the counter never increments here. Remove lines 675-677.

### A175-03 - crates/incular-wgpu/src/renderer/backdrop.rs:37 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
pub(super) fn lower_backdrop(
```
Suggested fix: Replace with #[expect(clippy::too_many_arguments, reason = "lowering takes the full backdrop context; grouping it into a struct is tracked separately")]. The same change applies to the #[allow(clippy::too_many_arguments)] at backdrop.rs:162 on execute_backdrop.

### A175-02 - crates/incular-wgpu/src/renderer/backdrop.rs:58 (§2, medium)
Evidence:
```
let mut stages = Vec::new();
for stage in filter.stages() {
```
Suggested fix: Per frame, per backdrop with an image filter, this allocates twice: filter.stages() (incular-rendering/src/effects.rs:874) returns a Vec, and the local Vec is then converted with stages.into() into the Arc<[BackdropStage]>. Build the Arc directly from one allocation, e.g. collect the mapped stages into the final slice (or have the filter expose an iterator instead of a Vec), and reserve capacity if the count is known.

### A175-04 - crates/incular-wgpu/src/renderer/backdrop.rs:369 (§26, low)
Evidence:
```
let [a, b, c, d, e, f] = sampling.to_kurbo().as_coeffs().map(|value| value as f32);
let mut weights = [0.; BLUR_WEIGHT_SLOTS];
weights[..6].copy_from_slice(&[a, b, c, d, e, f]);
```
Suggested fix: render_backdrop_matrix builds the same GpuBlurParams as the free function matrix_params (backdrop.rs:419-437), which does the identical coefficient conversion, weight fill and field set. Replace the inline block with: let params = matrix_params(sampling, size, size); (where size = [width as f32, height as f32]) and keep the [a..f] destructuring only for matrix_bits. Removes the duplicated parameter construction.

### A180-03 - crates/incular-wgpu/src/renderer/lowering.rs:23 (§11, low)
Evidence:
```
let clip = *clips.last().expect("clip stack");
...
append_rrect(
    &mut batches,
    *clips.last().expect("clip stack"),
```
Suggested fix: The clip-stack top is re-read in about 15 arms (lines 23, 80, 95, 113, 125, 132, 212, 219, 242, 302, 337, 368, 439, 479, 523, 563). Read it once at the top of the loop next to `let transform = ...` (`let clip = *clips.last().expect("clip stack");`) and use `clip` in those arms. Arms that push or pop clips must re-read after the mutation, so compute it before the match and refresh it only in the PushClip*/PopClip arms.

### A180-02 - crates/incular-wgpu/src/renderer/lowering.rs:251 (§2, medium)
Evidence:
```
let child = self.lower_commands(
    &commands[start..end],
    scale,
```
Suggested fix: Each nested lowering returns a freshly allocated Vec<DrawBatch> that the caller then copies with `batches.extend(child)`. Change `lower_commands` to take `batches: &mut Vec<DrawBatch>` (append into the caller's output) and call `self.lower_commands(&commands[start..end], scale, transform, parent_clip, batches)?;` at this site. The same change applies to the child calls at lines 265-272 (`batches.extend(child?)` after restoring content_opacity, so restore first then propagate the error), 306-312, 371-377, 445-451 and 507-509. Callers that build a temporary sub-list (render_offscreen_children, lower_shader_mask_fill) pass their own local Vec.

### A181-01 - crates/incular-wgpu/src/renderer/lowering.rs:1079 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
fn render_offscreen_children(
```
Suggested fix: #[expect(clippy::too_many_arguments, reason = "the offscreen group parameters are forwarded unchanged to lower_commands and the cache entry; bundling them would add a per-call struct")]


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
