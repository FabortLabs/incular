# Fix brief incular-widgets-17

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/advanced_slivers/tree_sliver.rs`, `crates/incular-widgets/src/devtools.rs`, `crates/incular-widgets/src/painting_effects/box_painter.rs`. Other agents are editing other files at
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
### A196-01 - crates/incular-widgets/src/advanced_slivers/tree_sliver.rs:698 (§2, high)
Evidence:
```
let next_active = self.state.borrow().active.clone();
let changed = self.active.len() != next_active.len()
```
Suggested fix: Compare against the borrowed state without cloning and clone only when changed: let state = self.state.borrow(); let changed = self.active.len() != state.active.len() || self.active.iter().zip(&state.active).any(|(l, r)| l.id != r.id || l.depth != r.depth); if changed { ...; self.active.clear(); self.active.extend_from_slice(&state.active); ... }. This removes a Vec allocation on every layout pass.

### A196-06 - crates/incular-widgets/src/advanced_slivers/tree_sliver.rs:778 (§11, medium)
Evidence:
```
.iter()
.position(|row| row.id == animation.parent_id)?;
(parent_position < position).then_some(
```
Suggested fix: animation_shift (called once per visible row at line 849) and row_animation (line 753) each rescan self.active and the animations list. That makes each frame O(visible rows x animations x active rows). Resolve each snapshot's parent position once per layout, store it as parent_position: Option<usize> on TreeAnimationSnapshot (or in a reused buffer), and have animation_shift and row_animation read that field.

### A196-05 - crates/incular-widgets/src/advanced_slivers/tree_sliver.rs:812 (§3, medium)
Evidence:
```
let row = self.active.iter().find(|row| row.id == *id)?;
Some(self.extent_at(
    self.active.iter().position(|item| item.id == row.id)?,
```
Suggested fix: Scan once: let position = self.active.iter().position(|row| row.id == *id)?; Some(self.extent_at(position, &self.active[position])). The same node is currently searched twice per direct child, and this runs once per animating parent.

### A196-07 - crates/incular-widgets/src/advanced_slivers/tree_sliver.rs:843 (§11, medium)
Evidence:
```
let node = state.find_node(row.id);
let Some(node) = node else {
```
Suggested fix: find_node is a recursive whole-tree search, so each visible row costs O(total nodes) per frame. Record each active row's child-index path in flatten_nodes into a flat reused buffer, then resolve nodes by walking that path, so lookup is O(depth). extent_at (line 730) has the same cost for unmeasured rows and should use the same lookup.

### A206-03 - crates/incular-widgets/src/devtools.rs:98 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
fn walk_dev(
    &self,
```
Suggested fix: walk_dev takes six parameters including self, below clippy's default threshold of seven, so the allow is stale. Delete the attribute. If a future change does trip the lint, use `#[expect(clippy::too_many_arguments, reason = "...")]` instead.

### A206-02 - crates/incular-widgets/src/devtools.rs:127 (§11, high)
Evidence:
```
let child_ids = self.dev_visible_children(id, include_internal);
...
for child in self.dev_visible_child_elements(id, include_internal) {
```
Suggested fix: Compute the visible child elements once and derive the ids from them: `let children = self.dev_visible_child_elements(id, include_internal);` then `child_ids: children.iter().map(|&c| dev_id(c)).collect(),` (or push ids into a reused buffer) and `for child in children {` for the recursion. Today each element allocates twice (dev_visible_children and dev_visible_child_elements), and dev_visible_child_elements (line 151, `let mut visible = Vec::new();`) allocates again on every recursive call for internal wrappers. A single accumulator `&mut Vec<ElementId>` passed through dev_visible_child_elements removes the recursive allocations.

### A206-01 - crates/incular-widgets/src/devtools.rs:135 (§2, high)
Evidence:
```
let child_ids = self.dev_visible_children(id, include_internal);
out.push(WidgetNode {
    child_ids: child_ids.clone(),
```
Suggested fix: Drop the clone and move the vector: `child_ids: child_ids,` (or `child_ids,`). `child_ids` is not used after this push, so the per-node Vec allocation disappears. See the companion finding at line 127 for computing the list once.

### A206-06 - crates/incular-widgets/src/devtools.rs:880 (§18, low)
Evidence:
```
#[cfg_attr(not(feature = "devtools"), allow(dead_code))]
pub(crate) fn map_cause(cause: &InvalidationCause) -> InvalidationReason {
```
Suggested fix: Change to `#[cfg_attr(not(feature = "devtools"), expect(dead_code, reason = "only the devtools feature maps causes for snapshots"))]`. The expect is fulfilled in the non-devtools build, where map_cause is unused, so the lint suppression is still exact.

### A242-04 - crates/incular-widgets/src/painting_effects/box_painter.rs:312 (§2, low)
Evidence:
```
let mut layers: Vec<Widget> = decoration
    .box_shadow
    .iter()
```
Suggested fix: Reserve the known final length (shadows plus up to three pushed layers) so the later `push` calls do not reallocate: `let mut layers = Vec::with_capacity(decoration.box_shadow.len() + 3); layers.extend(decoration.box_shadow.iter().map(|shadow| styled_shadow(&decoration, size, shadow)));`.

### A242-01 - crates/incular-widgets/src/painting_effects/box_painter.rs:710 (§2, medium)
Evidence:
```
let mut tiles = Vec::new();
for i in start_x..=stop_x {
    for j in start_y..=stop_y {
```
Suggested fix: Size the vector from the known tile count (capped at the 16_384 limit): `let count = (i64::from(stop_x) - i64::from(start_x) + 1).max(0) * (i64::from(stop_y) - i64::from(start_y) + 1).max(0);` then `let mut tiles = Vec::with_capacity(count.min(16_384) as usize);`. Alternatively return `impl Iterator<Item = Rect>` built from `(start_x..=stop_x).flat_map(...).take(16_384)` so no Vec is needed at all.

### A242-02 - crates/incular-widgets/src/painting_effects/box_painter.rs:867 (§2, medium)
Evidence:
```
let tiles = if repeat == crate::ImageRepeat::NoRepeat {
    vec![destination_rect]
} else {
```
Suggested fix: Avoid the one-element Vec on every image paint. Move the per-tile body of the `match paint.center_slice` (lines 872-896) into a closure `let mut paint_tile = |tile: Rect| match paint.center_slice { None => canvas.image_with_sampling(paint.image.clone(), source_rect, tile, paint.sampling), Some(_) => paint_nine(&mut canvas, paint, center, tile, pixels) };` (hoisting `source_rect`/`center` out of the match), then `if repeat == crate::ImageRepeat::NoRepeat { paint_tile(destination_rect); } else { for tile in image_tile_rects(rect, destination_rect, repeat) { paint_tile(tile); } }`. Works together with the iterator change suggested for `image_tile_rects`.

### A242-03 - crates/incular-widgets/src/painting_effects/box_painter.rs:957 (§2, medium)
Evidence:
```
let blurred = Widget::blur(sigma, shape());
let mask = match shadow.blur_style {
    BlurStyle::Outer => Widget::blend(incular_rendering::BlendMode::DstOut, shape()),
```
Suggested fix: Build the opaque shape once and clone the Widget handle (`Widget::clone` is one Rc clone): `let shape: Widget = CustomPaint::new(size, decoration.paint_shadow(size, &opaque)).into();` then `let blurred = Widget::blur(sigma, shape.clone());` and use `shape.clone()` in the Outer and Inner arms and `shape` in the Solid arm. Removes a second full `paint_shadow` DisplayList build for every non-normal blur style shadow in each layout.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
