# Fix brief incular-text-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-text/src/engine.rs`, `crates/incular-text/src/editing.rs`. Other agents are editing other files at
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
### A157-04 - crates/incular-text/src/engine.rs:382 (§2, low)
Evidence:
```
fn derive_caret_positions(&self) -> Arc<[TextCaretPosition]> {
    let mut positions = Vec::new();
    for index in 0..self.lines.len() {
```
Suggested fix: Size the buffer up front from the known cluster counts (two stops per cluster plus up to two per line):
let mut positions = Vec::with_capacity(
    self.lines.iter().map(|line| line.clusters.len() * 2 + 2).sum(),
);

### A158-09 - crates/incular-text/src/engine.rs:1347 (§5, low)
Evidence:
```
let mut names = vec![family_name(&style.family)];
```
Suggested fix: font_stack builds one String per family_name plus a Vec and a joined String on every call, and for_each_style_property calls it for each styled run on each shape. Write the stack into one String with write! (or a single with_capacity buffer) instead of Vec<String> plus join.

### A158-02 - crates/incular-text/src/engine.rs:1434 (§2, low)
Evidence:
```
let mut candidate = text[..end].to_owned();
```
Suggested fix: Hoist the buffer out of the loop and reuse it: let mut candidate = String::with_capacity(text.len() + 3); then inside the loop candidate.clear(); candidate.push_str(&text[..end]); candidate.push('\u{2026}');

### A158-05 - crates/incular-text/src/engine.rs:1539 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
```
Suggested fix: #[expect(clippy::too_many_arguments, reason = "shape_raw mirrors the TextLayoutOptions fields it forwards to Parley; bundling them would copy them again per call")]

### A159-03 - crates/incular-text/src/engine.rs:1659 (§2, low)
Evidence:
```
let mut lines = Vec::new();
let mut font_runs = Vec::new();
let mut used_fonts = HashSet::new();
```
Suggested fix: let mut lines = Vec::with_capacity(layout.len()); (the line count is known from the Parley layout, as used at line 1648). Leave font_runs and used_fonts as they are, since their length is not known up front.

### A159-01 - crates/incular-text/src/engine.rs:1837 (§2, medium)
Evidence:
```
let visible: Vec<_> = parley_run.glyphs().collect();
let all: Vec<_> = clustered().map(|(glyph, _)| glyph).collect();
all.windows(visible.len())
```
Suggested fix: Remove both collects and search without heap: let n = parley_run.glyphs().count(); let first_glyph = (0..=clustered().count().saturating_sub(n)).find(|&s| clustered().skip(s).map(|(glyph, _)| glyph).take(n).eq(parley_run.glyphs())).unwrap_or(0); Same first-window semantics as windows(n).position(), no per-run Vec allocations on the split-run path.

### A159-02 - crates/incular-text/src/engine.rs:1879 (§2, medium)
Evidence:
```
let cluster_spans: Vec<_> = merge_ligature_components(clusters.positions.drain(..))
    .iter()
    .map(|position| TextClusterSpan {
```
Suggested fix: Collect straight into the shared slice: let cluster_spans: Arc<[TextClusterSpan]> = merge_ligature_components(clusters.positions.drain(..)).iter().map(|position| TextClusterSpan { ... }).collect(); and set clusters: cluster_spans, at line 1900 instead of cluster_spans.into(). The slice Map iterator is TrustedLen, so this is one allocation per line instead of Vec plus a copy into Arc.

### A159-04 - crates/incular-text/src/engine.rs:2194 (§2, low)
Evidence:
```
let lines = layout.lines.iter().take(limit).cloned().collect::<Vec<_>>();
if let Some(carets) = layout.caret_positions.get_mut() {
```
Suggested fix: let lines: Arc<[TextLine]> = layout.lines.iter().take(limit).cloned().collect(); (Take<Cloned<slice::Iter>> is TrustedLen, so one allocation). Then at line 2206 replace layout.lines = lines.into(); with layout.lines = lines; The later lines.len(), lines.first() and lines.iter() calls work unchanged on the Arc slice. This removes the Vec-to-Arc copy in the clipped overflow path.

### A159-05 - crates/incular-text/src/engine.rs:2345 (§2, low)
Evidence:
```
for placed in Arc::make_mut(&mut translated.placeholders) {
    if let (Some(line), Some((baseline, _))) =
        (lines.get(placed.line), shifts().nth(placed.line))
```
Suggested fix: Walk the shifts once instead of rescanning from the first line for every placeholder. Placeholders are pushed in ascending line order (translate_layout extends placeholders per line), so keep one cursor: let mut shifts = shifts().enumerate().peekable(); and for each placed, advance the cursor while its index is below placed.line, then read the matching (baseline, _). This makes the placeholder pass O(placeholders + lines) instead of O(placeholders * lines), with no allocation.

### U106-03 - crates/incular-text/src/editing.rs:822 (§2, high)
Evidence:
```
let mut value = self.value();
        let target = nearest_char_boundary(&value.text, target.min(value.text.len()));
        value.selection = if extend {
```
Suggested fix: Do not clone the text to change only the selection. Read base and the clamped target with with_value, then write the selection with update_ranges as set_selection already does: let base = self.with_value(|value| value.selection.base); let target = self.with_value(|value| nearest_char_boundary(&value.text, target.min(value.text.len()))); let selection = if extend { TextSelection::new(base, target) } else { TextSelection::collapsed(target) }; self.update_ranges(|value| value.selection = selection); then keep the preferred_caret_x and caret_affinity assignments.

### U106-04 - crates/incular-text/src/editing.rs:836 (§2, high)
Evidence:
```
fn move_visual(&self, layout: &(impl TextGeometry + ?Sized), right: bool, extend: bool) {
        let value = self.value();
        let extent = value.selection.extent;
```
Suggested fix: Copy only the selection, not the whole text. Replace with: let selection = self.with_value(|value| value.selection); let extent = selection.extent; and then use selection instead of value.selection in the collapse branch (if !extend && !selection.is_collapsed() ...; selection.end(), selection.start()). Every arrow-key visual move otherwise clones the full document.

### U106-06 - crates/incular-text/src/editing.rs:1094 (§2, low)
Evidence:
```
Some(TextEditingValue::new(text.clone()).with_selection(selection.clamp_to(&text)))
```
Suggested fix: Clamp before moving the string, so the clone is not needed: let selection = selection.clamp_to(&text); Some(TextEditingValue::new(text).with_selection(selection))


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
