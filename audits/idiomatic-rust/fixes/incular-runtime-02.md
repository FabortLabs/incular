# Fix brief incular-runtime-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-runtime/src/frame/text_gestures.rs`, `crates/incular-runtime/src/restoration.rs`. Other agents are editing other files at
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
### A137-06 - crates/incular-runtime/src/frame/text_gestures.rs:44 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
pub(super) fn text_secondary_pointer(
```
Suggested fix: `#[expect(clippy::too_many_arguments, reason = "mirrors the pointer event fields Flutter's gesture builder receives")]`

### A137-04 - crates/incular-runtime/src/frame/text_gestures.rs:107 (§2, medium)
Evidence:
```
let Some(snapshot) = self.tree.text_field_input_snapshot(field) else {
    return false;
};
```
Suggested fix: Use the geometry-only snapshot, which does not copy the text: `let Some(snapshot) = self.tree.text_field_input_geometry(field) else {`. The secondary tap only reads `snapshot.traits`. The same full-text copy occurs in text_field_context_menu_items (line 316), which only uses `snapshot.text.len()` for text length; replace that length with `controller.with_value(|v| v.text.len())` and also switch to text_field_input_geometry.

### A137-05 - crates/incular-runtime/src/frame/text_gestures.rs:247 (§2, medium)
Evidence:
```
.selectable_text_selected_text(label)
.is_some_and(|text| !text.is_empty());
let can_select_all = self
```
Suggested fix: Both queries allocate a copy of the text only to test emptiness. Add a non-copying tree query (for example `selectable_text_has_text(label) -> bool` and `selectable_text_has_selection(label) -> bool` in incular-widgets tree/text.rs, built on the existing `selectable_text_value` / selection state) and use it here: `let can_select_all = self.tree.selectable_text_has_text(label);`. Keep the cloned text only in the dispatched Copy closure at line 261. Runs on each secondary click on selectable text.

### A137-02 - crates/incular-runtime/src/frame/text_gestures.rs:299 (§2, medium)
Evidence:
```
let text = controller.text();
let read_only = !self.tree.text_field_is_editable(field);
let (selection, affinity) = word_at_offset(
    &text,
```
Suggested fix: Read the text through the controller without copying: `let (selection, affinity) = controller.with_value(|value| word_at_offset(&value.text, offset, affinity, self.text_field_obscured(field), read_only, self.text_shortcut_platform));` (hoist obscured/read_only/platform into locals before the closure). Each double tap, long press and triple-tap path copies the whole field text otherwise.

### A137-01 - crates/incular-runtime/src/frame/text_gestures.rs:683 (§2, high)
Evidence:
```
let text = controller.text();
let obscured = self.text_field_obscured(field);
```
Suggested fix: Borrow the value instead of copying the whole field text on every drag move: `let (selection, affinity) = controller.with_value(|value| { let text = value.text.as_str(); let (from_word, affinity) = word_at_offset(text, from_offset, from_affinity, obscured, read_only, platform); let (to_word, _) = word_at_offset(text, to_offset, to_affinity, obscured, read_only, platform); let selection = if from_word.start() < to_word.end() { TextSelection::new(from_word.base, to_word.extent) } else { TextSelection::new(from_word.extent, to_word.base) }; (selection, affinity) }); self.tree.text_field_select(field, selection, affinity, now);` (compute obscured/read_only/platform before the closure, as today). Called from text_field_primary_drag and text_long_press_move, once per pointer move.

### A137-03 - crates/incular-runtime/src/frame/text_gestures.rs:717 (§2, medium)
Evidence:
```
let text = controller.text();
if !self.tree.is_multiline_text_field(field) {
    self.tree.text_field_select(
```
Suggested fix: Avoid the full-text copy: take `text.len()` via `controller.with_value(|v| v.text.len())` for the single-line branch, and for the multiline branch keep the `range` closure but compute it inside `controller.with_value(|value| ...)` over `value.text.as_str()`, or pass the length and only borrow text where `TextBoundaryKind::leading_at/trailing_at` need it. Verify that the tree's line-boundary queries do not take a mutable borrow of the controller before nesting them in with_value. Runs on triple taps and triple-tap drags.

### A142-03 - crates/incular-runtime/src/restoration.rs:995 (§2, low)
Evidence:
```
let mut segments = key.split('/').collect::<Vec<_>>();
let _ = segments.pop();
```
Suggested fix: The Vec exists only to drop the last segment. Use the same rule as scope_count and avoid the Vec and the join:
let scope = key.rsplit_once('/').map_or("application", |(scope, _)| scope);
then count with get_mut/insert on the &str (allocate the String only on first insert). Output is identical, including keys without '/' and leading-'/' keys.

### A142-05 - crates/incular-runtime/src/restoration.rs:1048 (§7, low)
Evidence:
```
) -> Result<SnapshotDocument, ()> {
let mut value: Value = serde_json::from_slice(bytes).map_err(|_| ())?;
```
Suggested fix: Return an error that says why a snapshot was rejected (parse, format/application id mismatch, migration failure) instead of unit, or Option<SnapshotDocument> if callers only count rejection. The call site already feeds diagnostics, so the cause can be recorded. Private helper, so no public API change.

### A142-04 - crates/incular-runtime/src/restoration.rs:1050 (§2, medium)
Evidence:
```
let mut value: Value = serde_json::from_slice(bytes).map_err(|_| ())?;
let document: SnapshotDocument = serde_json::from_value(value.clone()).map_err(|_| ())?;
```
Suggested fix: Deserialize from a borrow so the whole JSON tree is not cloned on every load:
let document = SnapshotDocument::deserialize(&value).map_err(|_| ())?;
(serde_json implements Deserializer for &Value; needs serde::Deserialize in scope.) value is still available for the migration branch below.

### A142-01 - crates/incular-runtime/src/restoration.rs:1073 (§2, medium)
Evidence:
```
path.iter()
.map(|segment| segment.as_str().replace('~', "~0").replace('/', "~1"))
.collect::<Vec<_>>()
```
Suggested fix: Encode in one pass into a single String, no per-segment replace allocations and no intermediate Vec:
fn encode_path(path: &[RestorationKey]) -> String {
    let mut out = String::new();
    for (index, segment) in path.iter().enumerate() {
        if index > 0 {
            out.push('/');
        }
        for ch in segment.as_str().chars() {
            match ch {
                '~' => out.push_str("~0"),
                '/' => out.push_str("~1"),
                _ => out.push(ch),
            }
        }
    }
    out
}
Same output. encode_path runs on every read/write/remove/acquire_scope, and read runs from from_scope during build.

### A142-02 - crates/incular-runtime/src/restoration.rs:1079 (§2, medium)
Evidence:
```
fn scope_count(values: &BTreeMap<String, Value>) -> usize {
values
.keys()
```
Suggested fix: Stop rebuilding a BTreeSet of every scope on each mutation. Keep a per-scope counter map next to snapshot.state, e.g. scope_counts: BTreeMap<String, usize>, updated in write (on insert of a new key), remove, remove_scope and reset (decrement, drop entry at zero), and have refresh_counts set restoration_scopes = scope_counts.len(). refresh_counts is called from write, remove, remove_scope and reset, each of which currently scans all keys and allocates a BTreeSet.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
