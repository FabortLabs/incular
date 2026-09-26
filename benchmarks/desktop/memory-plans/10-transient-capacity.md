# Recover oversized transient allocations

Status: proposed resource track after census 00.

## Finding and scope

Renderer state tracks six instance-buffer capacities, path caches, effect/offscreen
caches and reusable targets. Inspect `crates/incular-wgpu/src/renderer/state.rs`,
`frame.rs`, `rendering.rs`, `render_passes.rs` and `glyph_uploads.rs`. GlyphUploads
already releases CPU staging capacity at idle; preserve that completed optimization.
Inspect existing budgets before calling any cache unbounded. CPU image-cache limits
live in `crates/incular-image/src/lib.rs` if measurements implicate them.

## Implementation sequence

1. Inventory owner, requested bytes, capacity, demand/peak, lifetime and existing
   eviction for every candidate. Separate CPU scratch, GPU resources, allocator
   reservations and opaque driver allocations.
2. Probe burst then recovery: large text, paths/effects, resize up/down, route close
   and multi-window teardown. Identify actual avoidable high-water retention.
3. Prototype conservative hysteresis with minimum capacity, sustained low-demand
   threshold and cooldown. Derive thresholds from workloads, not an idle score.
4. Use existing maintenance/lifecycle/pressure opportunities. Do not add recurring
   idle timer wakeups solely to shrink memory; a no-frame idle state must stay idle.
5. Retire GPU buffers/targets only after submitted work can safely release them.
   Preserve bindings, device/window generations and protected live glyph pages.
6. Bound free offscreen targets by bytes and useful size classes where needed.
   Preserve active reuse and required effects/destination reads/stencil behavior.
   Never clear useful caches every frame.
7. Report live-resource reduction separately from backend-reserved and process
   memory. Dropped resources may leave WGPU/driver blocks reserved.

## Evidence, validation and acceptance

Record demand/capacity, GPU live/reserved bytes, process resident/commit, frame
latency and idle CPU before burst, at peak and after recovery. Do not sum separate
accounting domains. Include renewed interaction to detect reallocation thrash.

Use affected glyph_uploads, shared-resource, renderer and frame-outcome tests.
Run native resize/effect/multi-window coverage only where relevant. Require safe
burst/idle/reuse cycles, unchanged captures and no allocation oscillation.

Accept recovery of oversized unused allocations without GPU lifetime errors,
pixel changes or extra wakeups. Reject working-set trimming, benchmark-only purges
and removing the AMD CPU workaround. Deliver owner inventory, threshold rationale,
isolated patches and protocol-00 results. Preserve existing batching/font sharing.
