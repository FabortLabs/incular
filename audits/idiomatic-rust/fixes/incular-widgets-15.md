# Fix brief incular-widgets-15

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/painting/behavior/content.rs`, `crates/incular-widgets/src/advanced_slivers/animated.rs`. Other agents are editing other files at
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
### A292-06 - crates/incular-widgets/src/tree/painting/behavior/content.rs:37 (§2, medium)
Evidence:
```
for rect in selection_rects(&layout, selection, 0., 0., 0.) {
cache.push(PaintCommand::Rect {
rect,
```
Suggested fix: selection_rects (tree/text.rs:1799) returns Vec<Rect> and is allocated every frame while a SelectionArea selection covers a Text. Make it return `impl Iterator<Item = Rect> + '_` (the body is already a flat_map chain), or push straight into the DisplayList with `&mut DisplayList`. The same applies to the call at line 220 inside the TextField arm.

### A292-02 - crates/incular-widgets/src/tree/painting/behavior/content.rs:106 (§2, high)
Evidence:
```
for destination in image_repeat_destinations(
destination,
Rect::from_origin_size(Offset::ZERO, size),
```
Suggested fix: image_repeat_destinations (tree/rendering.rs:321) returns Vec<Rect> and allocates `vec![destination]` even for ImageRepeat::NoRepeat, so every painted Image allocates every frame. Either return `impl Iterator<Item = Rect>` from it, or in this arm add a non-allocating fast path: `if matches!(repeat, ImageRepeat::NoRepeat) { cache.push(PaintCommand::Image { image: image.clone(), source, destination, sampling }); } else { for destination in image_repeat_destinations(...) { ... } }`.

### A292-03 - crates/incular-widgets/src/tree/painting/behavior/content.rs:157 (§2, high)
Evidence:
```
let shows_placeholder =
with_text_field_display(controller, placeholder, obscure_text, |display| {
display == placeholder.as_str()
```
Suggested fix: Every frame for every TextField this builds the display string (`"*".repeat(len)` for obscured fields, a full text clone while preediting) only to compare it with the placeholder. Put the cheap test first so the display is never built for non-empty values: `let shows_placeholder = value_empty && with_text_field_display(controller, placeholder, obscure_text, |display| display == placeholder.as_str());`. Both operands are pure, so the result is identical.

### A292-01 - crates/incular-widgets/src/tree/painting/behavior/content.rs:403 (§2, high)
Evidence:
```
let mut segment: Vec<incular_rendering::GlyphPosition> = Vec::new();
for glyph in run.glyphs.iter() {
cache.push(PaintCommand::GlyphRun {  // via push_glyph_segment: glyphs.into()
```
Suggested fix: Segments are contiguous ranges of run.glyphs, so no Vec is needed: track `start` and the segment colour, and on a colour change or at run end emit `Arc::from(&run.glyphs[start..i])` (one allocation, no per-run Vec, no Vec->Arc double copy). Change push_glyph_segment to take `glyphs: &[GlyphPosition]` and do `glyphs: Arc::from(glyphs)`. The same Vec-then-into pattern repeats in push_colored_line_runs (lines 459-470) and should get the same change.

### A292-05 - crates/incular-widgets/src/tree/painting/behavior/content.rs:492 (§2, medium)
Evidence:
```
let spans = line.selection_spans(run.range.start, run.range.end);
if spans.is_empty() {
continue;
```
Suggested fix: TextLineRef::selection_spans (incular-text engine.rs:94 / geometry.rs:218) returns a fresh Vec<(f32, f32)> for every decorated run on every line each frame. Add a visitor or buffer-filling form in incular-text (for example `selection_spans_into(&self, start, end, out: &mut Vec<(f32, f32)>)`, or a closure form) and reuse one scratch Vec owned by the paint path, cleared per call. The fix lives in incular-text; this call site is where the cost is paid.

### A292-04 - crates/incular-widgets/src/tree/painting/behavior/content.rs:502 (§2, medium)
Evidence:
```
let mut positions = Vec::with_capacity(3);
if decoration.contains(TextDecoration::UNDERLINE) {
positions.push(baseline + font_size * 0.1);
```
Suggested fix: Allocates per decorated run per line per frame. Use a fixed array with a count (at most three positions): `let mut positions = [0.0_f32; 3]; let mut count = 0;` and `positions[count] = ...; count += 1;`, then iterate `&positions[..count]`.

### A194-02 - crates/incular-widgets/src/advanced_slivers/animated.rs:619 (§2, high)
Evidence:
```
self.widgets.insert(entry.id, widget.clone());
widget
```
Suggested fix: The `widgets: HashMap<u64, Widget>` field is write-only: nothing in the file reads it (only init at line 572, `retain` at line 584 and this insert). Delete the field, its initializer at line 572, the `self.widgets.retain(...)` line 584, and this insert. `widget_for` then just returns the built widget: `if entry.is_outgoing() { ... } else { (self.builder)(entry.index, entry) }`. This removes one Widget clone and one HashMap insert per materialized child per layout.

### A195-02 - crates/incular-widgets/src/advanced_slivers/animated.rs:691 (§11, medium)
Evidence:
```
let scroll_extent = (0..self.entries.len())
    .map(|position| self.list_entry_extent(position))
    .sum::<f32>()
```
Suggested fix: Recomputes the total extent over all entries every layout even when nothing changed. Read the total from self.index (it already tracks measured extents and revision) or cache it with the same revision/dirty check used for structure changes.

### A195-06 - crates/incular-widgets/src/advanced_slivers/animated.rs:741 (§11, medium)
Evidence:
```
for column in 0..columns {
    let position = row * columns + column;
    if position < count {
```
Suggested fix: Iterates every column of every row even when the row is past the last item, so a large column count (MaxCrossAxisExtent with a small extent on a wide viewport) costs columns * rows per frame. Bound the inner loop: `let first = row * columns; for position in first..(first + columns).min(count) { positions.push(position); }`.

### A195-04 - crates/incular-widgets/src/advanced_slivers/animated.rs:802 (§19, medium)
Evidence:
```
match &self.layout.clone() {
    CollectionLayout::List => self.list_layout(constraints),
```
Suggested fix: The clone exists only to dodge the &mut self borrow in list_layout/grid_layout, and runs every layout. Make CollectionLayout and SliverGridDelegate Copy (they hold plain numbers and enums) and write `match self.layout {` / `CollectionLayout::Grid(delegate) => self.grid_layout(constraints, delegate),`, or copy the delegate out with `*delegate` before the call.

### A195-05 - crates/incular-widgets/src/advanced_slivers/animated.rs:816 (§10, medium)
Evidence:
```
let Some(position) = self.entries.iter().position(|entry| entry.id == child.0) else {
```
Suggested fix: Linear scan per child extent report (called for each measured child, so O(n^2) per frame). Maintain an id-to-position map alongside `entries` and rebuild it in sync_entries when the structure revision changes, then `let Some(&position) = self.positions_by_id.get(&child.0) else { return false; };`.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
