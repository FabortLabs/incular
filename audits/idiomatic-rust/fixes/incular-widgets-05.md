# Fix brief incular-widgets-05

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/scrolling/layout.rs`. Other agents are editing other files at
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
### A258-05 - crates/incular-widgets/src/scrolling/layout.rs:152 (§10, low)
Evidence:
```
if let Some(widget) = self.widgets.get(&index) {
    return widget.clone();
}
let widget = (self.builder)(index);
self.widgets.insert(index, widget.clone());
```
Suggested fix: Use one hash lookup on the miss path: `self.widgets.entry(index).or_insert_with(|| (self.builder)(index)).clone()`. The same get-then-insert pattern repeats in GridRenderSliver::child_widget (lines 238-243) and VariableExtentRenderSliver::child_widget (lines 371-376), so apply it to all three.

### A258-04 - crates/incular-widgets/src/scrolling/layout.rs:300 (§23, high)
Evidence:
```
Widget::from(
SizedBox::new()
.width(cell_cross_extent)
```
Suggested fix: GridRenderSliver::perform_layout rebuilds a SizedBox wrapper per cell (lines 300-312), a Row or Column per row (lines 316-318) and a SliverChildLayout per row on every layout, which runs on every scroll frame. Cache the wrapped cell Widget next to its child in `widgets`, and cache each row widget, keyed by (row, columns, main_extent, cross_spacing, cell_cross_extent, axis). Rebuild only when the key changes, and store the key on the sliver.

### A258-03 - crates/incular-widgets/src/scrolling/layout.rs:426 (§23, medium)
Evidence:
```
let mut kept: Vec<usize> = self
    .widgets
    .keys()
```
Suggested fix: Per layout this builds `kept`, then `leading` and `trailing` through partition (lines 426-434) and allocates a fresh Vec each frame whenever keep-alive rows exist. Store `kept: Vec<usize>` as a scratch field, `clear()` and `extend(self.widgets.keys().copied().filter(|i| !range.contains(i)))`, `sort_unstable()`, then `let split = kept.partition_point(|i| *i < range.start);` and iterate `kept[..split]` as leading and `kept[split..]` as trailing. Take the field with std::mem::take while building children, then restore it.

### A258-01 - crates/incular-widgets/src/scrolling/layout.rs:437 (§23, high)
Evidence:
```
let edges = self.index.offsets_for_range(index..index + 1);
children.push(self.variable_child_layout(index, edges[0], edges[1], constraints));
(same pattern again at line 446 for trailing rows)
```
Suggested fix: Stop allocating a Vec<f32> per kept row. Add a non-allocating accessor to MeasuredExtentIndex in crates/incular-scroll/src/extent_index.rs, e.g. `pub fn edges(&self, index: usize) -> (f32, f32)`, and use `let (start, end) = self.index.edges(index); children.push(self.variable_child_layout(index, start, end, constraints));` at lines 437-438 and 446-447.

### A258-02 - crates/incular-widgets/src/scrolling/layout.rs:441 (§23, high)
Evidence:
```
let offsets = self.index.offsets_for_range(range.clone());
for (index, edges) in range.zip(offsets.windows(2)) {
    children.push(self.variable_child_layout(index, edges[0], edges[1], constraints));
```
Suggested fix: offsets_for_range (crates/incular-scroll/src/extent_index.rs:405) returns a fresh Vec<f32> on every layout. Keep a scratch `offsets: Vec<f32>` field on VariableExtentRenderSliver, take it with std::mem::take, fill it through a buffer-taking variant (`offsets_for_range_into(range, &mut Vec<f32>)`, the private helper at extent_index.rs:224 already has this shape), then put it back. Clear and reuse it each layout.

### A259-01 - crates/incular-widgets/src/scrolling/layout.rs:1170 (§11, high)
Evidence:
```
children: vec![SliverChildLayout {
    id: SliverChildId(0),
    widget: self.child.clone(),
```
Suggested fix: Every render sliver's perform_layout returns a freshly allocated Vec for SliverLayout::children (geometry.rs:115), so each sliver allocates once per layout. Change the field in src/scrolling/geometry.rs to `pub children: smallvec::SmallVec<[SliverChildLayout; 1]>` (one-child slivers stay inline) and write `children: smallvec![SliverChildLayout { ... }]` at each site: lines 1170, 1232, 1425, 1465 and 1691. For the many-child viewport (line 1565-1583) keep a reusable `Vec<SliverChildLayout>` buffer on ViewportExtentRenderSliver, clear and refill it each layout, and hand it out instead of `.collect()`. Verify that the sequence consumes the layout by value or by reference, so the buffer can be returned without a copy.

### A259-03 - crates/incular-widgets/src/scrolling/layout.rs:1499 (§1, low)
Evidence:
```
pub(super) children: Option<Rc<Vec<Widget>>>,
```
Suggested fix: The shared immutable child list is `Rc<Vec<Widget>>`, which is a double indirection. Change to `pub(super) children: Option<Rc<[Widget]>>,` and convert once where the sliver is constructed with `Rc::from(vec)`. The `children.get(index).cloned()` call at line 1515 is unchanged.

### A259-02 - crates/incular-widgets/src/scrolling/layout.rs:1508 (§10, low)
Evidence:
```
if let Some(widget) = self.widgets.get(&index) {
    return widget.clone();
}
...
self.widgets.insert(index, widget.clone());
```
Suggested fix: On a cache miss the key is hashed twice (get, then insert), and the cache map is also swept every layout with `retain` (line 1564). Replace the lookup with one entry call: `self.widgets.entry(index).or_insert_with(|| self.children.as_ref().and_then(|c| c.get(index).cloned()).or_else(|| self.builder.as_ref().map(|b| b(index))).unwrap_or_else(|| Widget::from(SizedBox::shrink()))).clone()`. Disjoint field capture makes the closure legal under edition 2021 or later. The retained map is still keyed by index, so the retain sweep stays unless the window is moved to a dense ring buffer.

### A260-01 - crates/incular-widgets/src/scrolling/layout.rs:1866 (§11, high)
Evidence:
```
children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone(),
```
Suggested fix: Every NaturalHeaderRenderSliver::perform_layout allocates a one-element Vec (and clones the child Widget) per frame. Change SliverLayout::children to a SmallVec<[SliverChildLayout; 1]> (smallvec is already shared in the workspace) and build it with smallvec![SliverChildLayout { ... }], so single-child slivers stay inline. Keep the shared Widget clone only if it is a refcount clone.

### A260-02 - crates/incular-widgets/src/scrolling/layout.rs:2146 (§11, high)
Evidence:
```
children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone(),
```
Suggested fix: Same per-frame one-element Vec allocation as NaturalHeaderRenderSliver::perform_layout (line 1866). Resolved by the same SliverLayout::children change to SmallVec<[SliverChildLayout; 1]> with smallvec![...] at this site.

### A260-03 - crates/incular-widgets/src/scrolling/layout.rs:2267 (§11, high)
Evidence:
```
for child in &mut layout.children {
    child.widget = (self.wrap)(child.widget.clone());
}
```
Suggested fix: The wrap closure rebuilds every child widget on every layout pass even when the inner sliver is unchanged. Keep a per-sliver cache (for example a Vec<(SliverChildId, Widget)> field) and re-run (self.wrap) only when the inner revision() changed or the child id/widget differs from the cached entry; otherwise reuse the cached wrapped widget.

### A261-02 - crates/incular-widgets/src/scrolling/layout.rs:2897 (§2, medium)
Evidence:
```
let mut children = Vec::new();
...
children.extend(reverse.children.into_iter().map(|mut child| {
```
Suggested fix: Each centered layout allocates a fresh Vec for the output and then copies the reverse run's children into it, even though layout_run already returns an owned Vec. Take ownership of the reverse run's Vec instead: declare `let mut children: Vec<SliverChildLayout> = Vec::new();` before the `if has_reverse` block, and inside it use `children = reverse.children; for child in &mut children { child.offset = zero - child.offset - child.extent; }` in place of the extend/map. Before the forward extend, call `children.reserve(forward.children.len());`. The sort_by_key and the forward offset shift stay as they are. Output order and offsets are unchanged.

### A261-01 - crates/incular-widgets/src/scrolling/layout.rs:3306 (§2, medium)
Evidence:
```
.iter()
.map(|child| widget_main_extent_hint(child, axis))
.collect::<Option<Vec<_>>>()?;
```
Suggested fix: Replace the collected Vec with a direct loop that short-circuits the same way. Main-axis branch: `let mut total = 0.; for child in children { total += widget_main_extent_hint(child, axis)?; } let spacing = spacing.max(0.) * children.len().saturating_sub(1) as f32; finite(total + spacing)`. Cross-axis branch: `let mut max: Option<f32> = None; for child in children { let hint = widget_main_extent_hint(child, axis)?; max = Some(max.map_or(hint, |m| m.max(hint))); } max.or(Some(0.))`. The `?` inside the loop still returns None from widget_main_extent_hint, and empty children still give Some(0.) in both branches.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
