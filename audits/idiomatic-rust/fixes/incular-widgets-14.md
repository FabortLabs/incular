# Fix brief incular-widgets-14

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/layout.rs`, `crates/incular-widgets/src/tree/layout/behavior/containers.rs`. Other agents are editing other files at
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
### A285-02 - crates/incular-widgets/src/tree/layout.rs:135 (§2, low)
Evidence:
```
let renders = self
.renders
.iter()
```
Suggested fix: Collect exists only to dodge the borrow in mark_render_dirty, and it copies every render id in the tree. It runs only when application fonts change, so it is not steady-state. Either mark dirty through the existing iterator with a split borrow if mark_render_dirty can take the dirty-state storage directly, or reuse a tree-owned scratch Vec as in the finding above. Low priority.

### A285-01 - crates/incular-widgets/src/tree/layout.rs:150 (§2, medium)
Evidence:
```
let dirty = self
.tracked
.stateful_layout_builders
```
Suggested fix: Borrow-dodge collect runs on every layout pass and allocates whenever a local-state LayoutBuilder is dirty. Keep a tree-owned scratch buffer: add a `layout_scratch: Vec<RenderObjectId>` field to WidgetTree, then `let mut dirty = std::mem::take(&mut self.layout_scratch); dirty.clear(); dirty.extend(self.tracked.stateful_layout_builders.iter().filter_map(|id| { ... }));` keep the same filter_map body, then `for render in dirty.drain(..) { self.mark_render_dirty(render, DirtyFlags::LAYOUT | DirtyFlags::PAINT, true); }` and `self.layout_scratch = dirty;`. The same pattern exists in refresh_text_fields (line 174, `let pending: Vec<(RenderObjectId, bool)> = self`), so apply the same scratch-buffer fix there in one change.

### A286-01 - crates/incular-widgets/src/tree/layout.rs:583 (§11, high)
Evidence:
```
let mut path = Vec::new();
let mut cursor = Some(id);
while let Some(current) = cursor {
```
Suggested fix: Avoid the ancestor Vec entirely: walk upward once and fold the transform from the leaf side. Start with `let mut suffix = CoreTransform::IDENTITY;` and for the leaf use `suffix = CoreTransform::translation(node.offset)`; for each ancestor walked upward, `suffix = CoreTransform::translation(node.offset).then(self.child_content_transform(ancestor)).then(suffix)` (the leaf-side term gets no child_content_transform, mirroring the `index + 1 != path.len()` check). Verify the direction of `CoreTransform::then` before using it. Alternatively, keep a reusable scratch Vec on the tree and clear it per call. Either way no per-call allocation.

### A286-02 - crates/incular-widgets/src/tree/layout.rs:613 (§11, high)
Evidence:
```
let mut viewports = Vec::new();
let mut cursor = Some(id);
while let Some(current) = cursor {
```
Suggested fix: Drop the `viewports` Vec. Move `let mut viewport = Rect::from_origin_size(Offset::ZERO, self.root_layout_size().unwrap_or(Size::ZERO));` above the first `while let` loop and, at the point where `viewports.push(current)` runs (line 640), intersect immediately: `let rect = self.render_world_transform(current).transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, self.render_live(current, "viewport must remain live").size)); viewport = viewport.intersection(rect).unwrap_or(Rect::from_origin_size(rect.origin, Size::ZERO));`. The Vec is already filled leaf-to-root, the same order as the loop, so the folded result is identical. Also note `render_world_transform(id)` at line 650 is recomputed here for the same ancestor chain; reuse is possible once the path walk is cheap.

### A286-04 - crates/incular-widgets/src/tree/layout.rs:804 (§11, medium)
Evidence:
```
let next_y = self
    .renders
    .iter()
```
Suggested fix: Each pinned persistent header scans the whole `renders` arena (and calls `scroll_flow_position` for every matching header) on every call, and this function is reached once per ancestor from `render_origin`/`render_viewport_origin` and during paint. Restrict the search to the candidate headers of the same scroll: maintain a per-scroll list of pinned header ids (updated when headers are laid out or removed), or iterate only the children of `scroll` instead of all renders. Keep the `candidate == controller && axis && reverse && pinned` filter and the `position > flow_position` rule unchanged.

### A286-03 - crates/incular-widgets/src/tree/layout.rs:1030 (§11, high)
Evidence:
```
children => {
    many = children.to_vec();
    &many[..]
```
Suggested fix: Every layout of a node with three or more children allocates. Remove the copy by having `layout_kind` read the child ids by index from the retained tree (e.g. pass `id` and index into `self.renders.get(id.0)?.children[i]` on each access) instead of a borrowed slice, so the `single`/`pair`/`many` locals and the copy disappear. If `layout_kind` must keep a slice, use a tree-owned scratch stack (`Vec<RenderObjectId>` pushed/truncated per layout) and copy into a fixed buffer only when needed. Confirm `layout_kind` never mutates `children` storage it reads from before choosing the index form.

### A287-01 - crates/incular-widgets/src/tree/layout/behavior/containers.rs:65 (§11, high)
Evidence:
```
let flex_meta = children
    .iter()
    .map(|child| {
```
Suggested fix: Drop the per-layout Vec. Add a helper `fn flex_data(&self, child: RenderObjectId) -> (u32, FlexFit)` that does the render_live lookup and match, and call it in the first loop (line 79) and the flexible loop (line 106). Compute `total_flex` in the first loop by summing the helper's factors. Keep `flex_children` as a scratch Vec on WidgetTree (std::mem::take, clear, reuse, restore), so the steady state allocates nothing. Also see the `Vec::with_capacity` at line 77.

### A287-02 - crates/incular-widgets/src/tree/layout/behavior/containers.rs:77 (§11, high)
Evidence:
```
let mut flex_children = Vec::with_capacity(children.len());
let mut occupied_non_flex = 0.0;
for (child, (f, fit)) in children.iter().zip(&flex_meta) {
```
Suggested fix: Reuse a scratch `Vec<incular_layout::FlexChild>` stored on WidgetTree: take it with std::mem::take, clear it, push into it, pass `&scratch` to incular_layout::layout_flex, then store it back. Remove the per-layout with_capacity allocation. Pairs with the flex_meta finding at line 65.

### A287-03 - crates/incular-widgets/src/tree/layout/behavior/containers.rs:182 (§11, high)
Evidence:
```
let wrap_children = children
    .iter()
    .map(|child| {
```
Suggested fix: Hoist the buffer: keep a scratch `Vec<incular_layout::WrapChild>` on WidgetTree (std::mem::take, clear, extend, pass `&scratch` to layout_wrap, restore). This removes one allocation per Wrap layout. Do not change the constraints or the measurement order.

### A287-05 - crates/incular-widgets/src/tree/layout/behavior/containers.rs:217 (§2, medium)
Evidence:
```
let measured = children
    .iter()
    .map(|child| {
```
Suggested fix: Per Table layout there are three Vecs: `measured` (217), `widths` (230, returned by table_column_widths) and `cells` (236). Hoist them into WidgetTree scratch buffers (take, clear, refill, restore). The `cells` pass can reuse the `measured` buffer after the widths are computed, which reduces the Table to one scratch buffer. Keep the two-pass measurement order.

### A287-06 - crates/incular-widgets/src/tree/layout/behavior/containers.rs:646 (§2, medium)
Evidence:
```
let mut ids = HashMap::new();
for (index, &child) in children.iter().enumerate() {
    let render = self.parent_data_render(child, constraints)?;
```
Suggested fix: Avoid a fresh HashMap on every custom multi-child layout. Store a `HashMap<LayoutChildId, usize>` scratch on WidgetTree, clear it, fill it, and pass `&scratch`. At minimum use `HashMap::with_capacity(children.len())`. The `offsets.into_iter().collect()` at line 679 also re-collects the delegate's Vec; it can pass the Vec straight to ChildOffsets::from(Vec<Offset>) (tree/layout.rs:44).

### A287-07 - crates/incular-widgets/src/tree/layout/behavior/containers.rs:685 (§2, low)
Evidence:
```
let mut sizes = Vec::with_capacity(children.len());
for (index, &child) in children.iter().enumerate() {
    self.layout_render(
```
Suggested fix: Keep the sizes buffer in WidgetTree scratch storage (take, clear, refill, restore) so a Flow layout allocates nothing in the steady state. The Vec is also passed to FlowPaintingContext::new, so the scratch must be returned after paint_children runs.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
