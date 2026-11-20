# Fix brief incular-widgets-04

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/text.rs`. Other agents are editing other files at
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
### A304-06 - crates/incular-widgets/src/tree/text.rs:576 (§2, medium)
Evidence:
```
let text_len = controller.text().len();
let (index, affinity) =
layout
```
Suggested fix: `TextEditingController::text()` returns an owned String (incular-text/src/editing.rs:415), so this copies the whole field text on every pointer move during a drag just to take its length. Replace with `let text_len = controller.with_value(|value| value.text.len());`.

### A304-01 - crates/incular-widgets/src/tree/text.rs:1152 (§2, high)
Evidence:
```
pub(super) fn selectable_text_value(&self, id: ElementId) -> Option<String> {
WidgetKind::SelectableText { text, .. } | WidgetKind::Text { text, .. } => {
Some(text.clone())
```
Suggested fix: Return a borrow: `pub(super) fn selectable_text_value(&self, id: ElementId) -> Option<&str> { match self.elements.get(id.0)?.widget.kind() { WidgetKind::SelectableText { text, .. } | WidgetKind::Text { text, .. } => Some(text.as_str()), _ => None } }`. Read-only callers (static_selection_range at 1867, static_selection_text at 1922, selection_content_range at 1895-1901, which currently clones the whole text just for `.len()`) then borrow with no copy. Mutating methods that call `self.static_selections.insert`/`sync_static_selection` after reading the text (selectable_text_select, selectable_text_select_words_to, selectable_text_select_all, selectable_text_set_extent, selectable_text_move) add `.to_owned()` where the borrow of `self` would overlap, and selectable_text_editing_value returns `text.to_owned()` in its tuple.

### A305-01 - crates/incular-widgets/src/tree/text.rs:1152 (§2, high)
Evidence:
```
pub(super) fn selectable_text_value(&self, id: ElementId) -> Option<String> {
WidgetKind::SelectableText { text, .. } | WidgetKind::Text { text, .. } => {
Some(text.clone())
```
Suggested fix: Return a borrow: `pub(super) fn selectable_text_value(&self, id: ElementId) -> Option<&str> { match self.elements.get(id.0)?.widget.kind() { WidgetKind::SelectableText { text, .. } | WidgetKind::Text { text, .. } => Some(text.as_str()), _ => None } }`. Callers that only read length (1078-1079 `map_or(0, |text| text.len())`, 1898 `offset.saturating_add(tree.selectable_text_value(*entry)?.len())`) then stop cloning the whole text per entry. In selectable_text_select (994-1016), bind `let len = text.len();` before the `point` closure and use `len` in the closure, so the borrow of self ends before `self.static_selections.insert` (&mut self). Same for static_selection_range (1867) and static_selection_text (1922), which only slice the text.

### A305-03 - crates/incular-widgets/src/tree/text.rs:1171 (§6, low)
Evidence:
```
.then_some(vec![area])
.unwrap_or_default();
```
Suggested fix: `then_some` evaluates `vec![area]` eagerly, so the allocation happens even when the predicate is false. Use `if standalone { vec![area] } else { Vec::new() }`, or `.then(|| vec![area]).unwrap_or_default()`, where `standalone` is the existing `is_some_and(...)` result bound to a local.

### A305-05 - crates/incular-widgets/src/tree/text.rs:1186 (§2, medium)
Evidence:
```
pub(super) fn collect_selectable_texts(&self, id: ElementId, entries: &mut Vec<ElementId>) {
let mut work = vec![id];
```
Suggested fix: selectable_texts_in_area calls this once per child, and each call allocates a fresh `work` stack. Pass one stack in: `fn collect_selectable_texts(&self, id: ElementId, entries: &mut Vec<ElementId>, work: &mut Vec<ElementId>)` with `work.clear(); work.push(id);` and have selectable_texts_in_area own one `let mut work = Vec::new();` across all children (or push the children directly into a single shared stack).

### A304-02 - crates/incular-widgets/src/tree/text.rs:1231 (§11, high)
Evidence:
```
let entries = self.selectable_texts_in_area(selection.area);
static_selection_range(self, &entries, selection, element)
```
Suggested fix: Stop rebuilding the registrar list per call. `sync_static_selection` already computes `entries`; keep it on the per-area selection record (e.g. `StaticSelection` or a per-area `registrars: Vec<ElementId>` map updated in sync_static_selection and cleared when the area's registrations change) and have static_selection_range borrow it as `&[ElementId]`. Paint (painting/behavior/content.rs:34 and :60) calls this for every selectable element each frame while a selection exists, so the Vec from selectable_texts_in_area (line 1160) is a per-frame, per-element allocation. Interim: in selectable_texts_in_area line 1171 replace `.then_some(vec![area]).unwrap_or_default()` with `if matches!(...) { vec![area] } else { Vec::new() }` so the vec is not built on the non-selectable branch.

### A305-04 - crates/incular-widgets/src/tree/text.rs:1231 (§2, high)
Evidence:
```
let area = self.selection_area_ancestor(element)?;
let selection = self.static_selections.get(&area).copied()?;
let entries = self.selectable_texts_in_area(selection.area);
```
Suggested fix: static_selection_range (1228) builds a fresh Vec of every registered text in the area on each call (plus a text clone per entry via selectable_text_value). Callers that already hold `entries` use the free function directly, so this per-element method should either take the precomputed `&[ElementId]` from its caller or reuse a cached per-area registrar list invalidated by a generation counter, instead of allocating per call.

### A305-06 - crates/incular-widgets/src/tree/text.rs:1267 (§2, medium)
Evidence:
```
rects.extend(
selection_rects(&layout, range, 0., 0., 0.)
.into_iter()
```
Suggested fix: selection_rects (1799) collects into a Vec that is immediately consumed by `into_iter()`. Make it return `impl Iterator<Item = Rect> + 'a` (`fn selection_rects<'a>(layout: &'a dyn TextGeometry, ...) -> impl Iterator<Item = Rect> + 'a`) and write `rects.extend(selection_rects(&layout, range, 0., 0., 0.).map(|rect| transform.transform_rect_bbox(rect)));`. Its other caller (paint) can keep iterating or collect as needed.

### A304-03 - crates/incular-widgets/src/tree/text.rs:1347 (§11, high)
Evidence:
```
let areas = self
.tracked
.selection_boundaries
```
Suggested fix: refresh_selection_states runs on every layout pass (layout.rs:117 and reconciliation.rs:1985). It collects a new Vec of areas (line 1356) and calls sync_static_selection on each, which builds entries, selection geometry rects, content range and selection text even when nothing changed, and calls mark_render_paint on every registered element (lines 1222-1226). Add a per-area dirty flag set when a selection changes, registrations change or text changes, and only sync flagged areas; iterate the boundary set with an index loop or reusable scratch buffer instead of collect::<Vec<_>>(). Also skip the paint marking when the selection geometry is unchanged. Confidence is medium because it depends on how often layout runs in idle frames.

### A305-08 - crates/incular-widgets/src/tree/text.rs:1574 (§2, high)
Evidence:
```
} else if obscure_text {
f(&"*".repeat(value.text.len()))
```
Suggested fix: The doc comment says the common case borrows without copying, but an obscured field (every password field) allocates a mask string on every display call. Use a static mask and slice it: `const MASK: &str = "****************************************************************";` (or a size that covers typical inputs), then `if len <= MASK.len() { f(&MASK[..len]) } else { f(&"*".repeat(len)) }`. Mask glyphs are ASCII, so byte slicing is valid. Alternatively cache the mask on the render node keyed by byte length.

### A304-04 - crates/incular-widgets/src/tree/text.rs:1575 (§11, high)
Evidence:
```
} else if obscure_text {
f(&"*".repeat(value.text.len()))
```
Suggested fix: with_text_field_display builds a fresh mask String on every paint and layout pass of an obscured field. Keep the mask on the field's render state (e.g. a `mask: String` in TextFieldRender) and rebuild it only when the byte length changes: `if mask.len() != value.text.len() { mask.clear(); mask.extend(std::iter::repeat_n('*', value.text.len())); }`, then pass `&mask` to `f`. Pass the mask storage into with_text_field_display (or make the helper a method on the render node) so the closure still receives a &str.

### A304-08 - crates/incular-widgets/src/tree/text.rs:1582 (§3, low)
Evidence:
```
pub(super) fn valid_boundary(text: &str, offset: usize) -> usize {
let offset = offset.min(text.len());
text.char_indices()
```
Suggested fix: Use the std helper, which is a direct boundary search: `pub(super) fn valid_boundary(text: &str, offset: usize) -> usize { text.floor_char_boundary(offset) }`. The result is identical (largest char boundary <= offset, clamped to len), and it avoids iterating the characters. Called per element from static_selection_range on paint.

### A304-05 - crates/incular-widgets/src/tree/text.rs:1805 (§11, medium)
Evidence:
```
) -> Vec<Rect> {
layout
.lines()
```
Suggested fix: Return an iterator instead of a Vec: `pub(super) fn selection_rects<'a>(layout: &'a (dyn TextGeometry + 'a), selection: TextRange, scroll_x: f32, scroll_y: f32, top: f32) -> impl Iterator<Item = Rect> + 'a` and drop the trailing `.collect()` at line 1825. Callers then iterate directly: selection_geometry (line 1267) uses `selection_rects(...).map(|rect| transform.transform_rect_bbox(rect))` with `rects.extend(...)`, and paint (painting/behavior/content.rs:37 and :66) already loops with `for rect in`. This removes one Vec per selected element per paint.

### A305-02 - crates/incular-widgets/src/tree/text.rs:1834 (§11, medium)
Evidence:
```
let anchor_index = entries
.iter()
.position(|entry| *entry == selection.anchor.element)?;
```
Suggested fix: Compute the first/last entry indices once per selection geometry (in selection_geometry / static_selection_text, which already loop over entries) and pass them into a small struct (e.g. `SelectionSpan { first_index, last_index, first_byte, last_byte }`) instead of rescanning `entries` three times with `position` for every element. Currently selection_geometry (1253) and static_selection_text (1921) each call this per element, giving O(n^2) lookups per geometry or copy.

### A304-09 - crates/incular-widgets/src/tree/text.rs:1923 (§2, low)
Evidence:
```
.filter_map(|element| {
let range = static_selection_range(tree, entries, selection, *element)?;
let text = tree.selectable_text_value(*element)?;
Some(text[range.start..range.end].to_owned())
```
Suggested fix: Each selected element allocates an intermediate String that is only pushed into the result. Build the result directly: `let mut out = String::new(); for element in entries { if let (Some(range), Some(text)) = (static_selection_range(tree, entries, selection, *element), tree.selectable_text_value(*element)) { out.push_str(&text[range.start..range.end]); } } out`. Pair with the borrowed selectable_text_value from the first finding.

### A305-07 - crates/incular-widgets/src/tree/text.rs:1923 (§2, medium)
Evidence:
```
let text = tree.selectable_text_value(*element)?;
Some(text[range.start..range.end].to_owned())
.collect::<String>()
```
Suggested fix: After selectable_text_value returns `Option<&str>` (see the finding at line 1152), write `Some(&text[range.start..range.end])` and keep `.collect::<String>()`. This removes one String allocation per selected element; the output is unchanged.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
