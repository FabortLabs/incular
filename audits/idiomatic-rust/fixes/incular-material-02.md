# Fix brief incular-material-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-material/src/catalog/data_table.rs`, `crates/incular-material/src/inputs/text_fields.rs`. Other agents are editing other files at
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
### A082-04 - crates/incular-material/src/catalog/data_table.rs:394 (§2, low)
Evidence:
```
let mut columns: Vec<Widget> = Vec::new();
```
Suggested fix: The final column count is known up front: `let mut columns: Vec<Widget> = Vec::with_capacity(table.columns.len() + usize::from(checkboxes));`

### A082-02 - crates/incular-material/src/catalog/data_table.rs:485 (§2, low)
Evidence:
```
let mut heading: Vec<Widget> = Vec::new();
```
Suggested fix: The heading always has exactly three widgets, so build an array and skip the Vec: `let heading = if column.numeric { [arrow, spacer, label] } else { [label, spacer, arrow] };` then pass `Row::new(heading)` at line 518. Remove the `heading.extend(...)` branch at lines 508-512. `Row::new` already accepts arrays (see line 541).

### A082-03 - crates/incular-material/src/catalog/data_table.rs:727 (§3, low)
Evidence:
```
let rows = (first..(first + table.rows_per_page).min(count))
    .map(|index| {
```
Suggested fix: Collect nothing here: pass the mapped iterator straight to `DataTable::new`, which already collects it. Replace `let rows = ...collect::<Vec<_>>();` and `DataTable::new(table.columns.clone(), rows)` with `DataTable::new(table.columns.clone(), (first..(first + table.rows_per_page).min(count)).map(|index| { ... }))`.

### A082-05 - crates/incular-material/src/catalog/data_table.rs:727 (§21, low)
Evidence:
```
let rows = (first..(first + table.rows_per_page).min(count))
    .map(|index| {
        table.source.get_row(index).unwrap_or_else(|| {
```
Suggested fix: Use saturating arithmetic for page ends, since `rows_per_page` is caller-supplied and may be `usize::MAX` ("show all"): `first.saturating_add(table.rows_per_page)` here, at line 778 (`let last = ...`) and at line 838 (`page(first.saturating_add(per_page))`). Overflow only occurs when `first` is non-zero, e.g. after a rows-per-page change with a retained first row.

### A082-01 - crates/incular-material/src/catalog/data_table.rs:795 (§2, low)
Evidence:
```
let options = table.available_rows_per_page.clone();
let next = options
    .iter()
```
Suggested fix: Drop the clone and read the field in place: `let next = table.available_rows_per_page.iter().copied().find(|option| *option > per_page).unwrap_or(table.available_rows_per_page[0]);` (the `!is_empty()` guard in the `if let` already makes the index safe).

### A094-01 - crates/incular-material/src/inputs/text_fields.rs:754 (§2, medium)
Evidence:
```
let mut content: Widget = if let Some(decoration) = self.material_decoration.clone() {
            let decoration_theme = context
```
Suggested fix: Borrow instead of cloning: `if let Some(decoration) = &self.material_decoration {`. `InputDecoration::apply_defaults` takes `&self` and already returns an owned copy (see input.rs:664), so the clone on this line is a second, redundant whole-struct clone on every build.

### A094-02 - crates/incular-material/src/inputs/text_fields.rs:769 (§2, medium)
Evidence:
```
.empty(self.controller.text().is_empty())
                .enabled(self.enabled)
```
Suggested fix: Use the borrowing accessor: `.empty(self.controller.with_value(|value| value.text.is_empty()))`. `text()` clones the entire editor string only to test emptiness; the same clone is repeated at line 818 for the semantics value (keep that one, it needs an owned String).

### A094-04 - crates/incular-material/src/inputs/text_fields.rs:787 (§2, low)
Evidence:
```
if self.material_decoration.is_none()
            && (self.label_text().is_some()
```
Suggested fix: Change the helper to borrow: `fn label_text(&self) -> Option<&str> { self.semantic_label.as_deref() }`, use `self.label_text().is_some()` as-is, and at line 793 use `Text::new(label)` (Text::new takes impl Into<String>, so &str works). This removes a String clone per presence check and per use.

### A094-05 - crates/incular-material/src/inputs/text_fields.rs:1230 (§3, low)
Evidence:
```
for formatter in value.input_formatters {
            field = field.input_formatters([formatter]);
        }
```
Suggested fix: Replace the loop with one call: `field = field.input_formatters(value.input_formatters);`. `TextField::input_formatters` accepts `impl IntoIterator<Item = Rc<dyn TextInputFormatter>>` (text_fields.rs:592), and the order is preserved.

### A094-03 - crates/incular-material/src/inputs/text_fields.rs:1249 (§2, medium)
Evidence:
```
let decoration = validation_error
                .as_ref()
                .map_or(decoration.clone(), |error| decoration.error(error.clone()));
```
Suggested fix: Consume the values instead of cloning: `let decoration = match validation_error { Some(error) => decoration.error(error), None => decoration };`. The current `map_or` eagerly clones the whole InputDecoration even when there is no validation error, and clones the error string needlessly.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
