# Fix brief incular-wgpu-03

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-wgpu/src/renderer/state.rs`, `crates/incular-wgpu/src/resources.rs`, `crates/incular-wgpu/src/geometry.rs`, `crates/incular-wgpu/src/renderer/resources.rs`, `crates/incular-wgpu/build.rs`, `crates/incular-wgpu/src/glyph_rasterizer.rs`. Other agents are editing other files at
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
### A184-02 - crates/incular-wgpu/src/renderer/state.rs:91 (§23, medium)
Evidence:
```
pub(super) offscreen_cache: HashMap<incular_painting::LayerId, OffscreenCacheEntry>,
pub(super) effect_cache: HashMap<incular_painting::LayerId, EffectCacheEntry>,
```
Suggested fix: Declare both as crate::fast_hash::FastHashMap<incular_painting::LayerId, ...> (same as cpu_path_cache at line 55) and initialise them with Default::default() instead of HashMap::new() at lines 362-363. Keys are integer layer ids looked up per command, so the SipHash default is wasted work. Also apply to backdrop_capture_targets (line 114) and backdrop_capture_regions (line 117).

### A184-01 - crates/incular-wgpu/src/renderer/state.rs:236 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
pub(super) fn from_shared_pipeline_resources(
```
Suggested fix: #[expect(clippy::too_many_arguments, reason = "constructor takes each owned handle once and moves it into the struct literal")]

### A184-03 - crates/incular-wgpu/src/renderer/state.rs:272 (§26, low)
Evidence:
```
let capture_supported = config.usage.contains(wgpu::TextureUsages::COPY_SRC)
    && matches!(
        config.format,
```
Suggested fix: Drop the recomputation here and pass the already-computed `capture_supported` from new_with_shared (line 200) into from_shared_pipeline_resources, or extract the format match at lines 641-647 into a shared helper used by both sites so the format list is not duplicated.

### A185-02 - crates/incular-wgpu/src/resources.rs:310 (§23, medium)
Evidence:
```
if let Some(position) = self.lru.iter().position(|candidate| *candidate == id)
&& let Some(key) = self.lru.remove(position)
{
```
Suggested fix: Avoid the O(n) VecDeque scan on every touch (called once per shared texture used per frame, n up to the entry budget). Keep the recency tick in CachedTexture (already present as last_use) and index the LRU as `BTreeMap<u64, K>` (tick -> id): touch removes the old tick key and inserts the new one in O(log n); eviction uses `pop_first`. Also fold the three `entries` lookups in touch (get, contains_key, get_mut) into one `get_mut`. Update remove() and evict_excess() accordingly; the debug_assert on lru.len() still holds.

### A186-01 - crates/incular-wgpu/src/resources.rs:1379 (§11, medium)
Evidence:
```
            if let Some(generation) = image_textures.generation(id) {
                image_textures.touch(id, generation);
            }
```
Suggested fix: Look the generation up once and reuse it for the return value (lines 1379 and 1385 both call image_textures.generation(id) on the hit path; touch() never changes the generation, so the second lookup is redundant):
            let generation = image_textures.generation(id);
            if let Some(generation) = generation {
                image_textures.touch(id, generation);
            }
            return Ok(SharedImageAcquisition::new(resource, false, generation));
(gradient_resource at line 1557 already does this.)

### A186-02 - crates/incular-wgpu/src/resources.rs:1742 (§2, low)
Evidence:
```
        let dropped: Vec<ImageId> = self
            .image_owners
            .iter()
```
Suggested fix: Drop the collect-then-remove pass (it exists only to dodge the borrow of image_owners) and prune in place with retain using disjoint field borrows:
        let before = self.image_owners.len();
        let (images, registry, image_textures) = (&mut self.images, &mut self.registry, &mut self.image_textures);
        self.image_owners.retain(|id, owner| {
            if !owner.is_dropped() {
                return true;
            }
            images.remove(id);
            registry.remove_image(*id);
            image_textures.remove(*id);
            false
        });
        before - self.image_owners.len()
Note: called from sync_shared_textures (renderer/resources.rs:270) on every sync, so the full image_owners scan also runs per frame; the Vec only allocates when an entry actually drops.

### A169-01 - crates/incular-wgpu/src/geometry.rs:71 (§2, medium)
Evidence:
```
None => (path.bez_path().clone(), fill_rule),
```
Suggested fix: Borrow instead of cloning the whole BezPath; `outline` is only read (`flatten_contours(&outline)` already derefs). Add `use std::borrow::Cow;` and write:
let (outline, fill_rule) = match stroke {
    Some(stroke) => (Cow::Owned(stroke_outline(path.bez_path(), stroke)?), FillRule::NonZero),
    None => (Cow::Borrowed(path.bez_path()), fill_rule),
};

### A169-02 - crates/incular-wgpu/src/geometry.rs:122 (§2, medium)
Evidence:
```
let mut mesh = PathMesh::default();
for (position, miter) in interior.vertices {
    mesh.push(
```
Suggested fix: Capacity is known before the push loops: interior.vertices.len() plus two vertices per fringe contour point. Compute it up front and reserve, e.g.
let fringe: usize = contours.iter().map(Vec::len).sum();
let vertex_count = interior.vertices.len() + 2 * fringe;
let mut mesh = PathMesh {
    vertices: Vec::with_capacity(vertex_count),
    extrusions: Vec::with_capacity(vertex_count),
    coverage: Vec::with_capacity(vertex_count),
    indices: Vec::new(),
};
(then after `mesh.indices = interior.indices;` call `mesh.indices.reserve(6 * fringe);`)

### A183-02 - crates/incular-wgpu/src/renderer/resources.rs:158 (§10, low)
Evidence:
```
if self.gradient_cache.get(&key).is_some() {
    self.gradient_cache.record_use(key, self.counters.frames);
```
Suggested fix: Add a `contains_key(&self, id: &K) -> bool` method to the local cache type in crates/incular-wgpu/src/resources.rs (next to `get` at line 511) and write `if self.gradient_cache.contains_key(&key) {` here. The type exposes only `get`, so this is a readability fix, not a cost fix.

### A183-01 - crates/incular-wgpu/src/renderer/resources.rs:386 (§11, low)
Evidence:
```
let last_used = if is_source {
    self.offscreen_cache
        .get(&layer)
```
Suggested fix: Carry the timestamp out of the selection instead of re-looking it up. Change the candidate tuple to `(layer, is_source, last_used_frame)` by mapping `min_by_key` to `.map(|(layer, e)| (*layer, true, e.last_used_frame))` (and the same for effect_cache with `false`), compare the stored frames in the Some/Some branch, and delete the `let last_used = if is_source { .. get(&layer) .. }` block at lines 386-394 and the two `.get(...).map_or(u64::MAX, ..)` lookups at lines 365-372.

### A168-02 - crates/incular-wgpu/build.rs:35 (§5, low)
Evidence:
```
        bindings.push_str(&format!(
```
Suggested fix: Add `use std::fmt::Write as _;` and replace the push_str/format pair with `write!(bindings, "pub(crate) const {}: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{name}.naga\"));\n", name.to_ascii_uppercase()).expect("write to String cannot fail");`, removing the intermediate String allocation.

### A170-01 - crates/incular-wgpu/src/glyph_rasterizer.rs:182 (§2, medium)
Evidence:
```
segments: Vec::new(),
// per glyph: pen built in rasterize_indexed, then pushed to by every outline segment
```
Suggested fix: Keep a reusable scratch segment buffer in RasterFont (e.g. add `scratch: Mutex<Vec<Segment>>` next to `hinters`). In rasterize_indexed, take it with `std::mem::take` under the lock, call `clear()`, build the SegmentPen with `segments: scratch`, and store `pen.segments` back when done (on every return path). Alternatively pass `&mut Vec<Segment>` into `outline`. This removes the per-glyph Vec allocation and its growth reallocations on atlas misses.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
