# Fix brief incular-devtools-07

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `tools/incular-devtools/src/views/performance.rs`, `tools/incular-devtools/src/views/rendering.rs`, `tools/incular-devtools/src/views/shell.rs`, `crates/incular-devtools/src/commands.rs`, `crates/incular-devtools/src/discovery.rs`, `tools/incular-devtools/src/performance.rs`, `tools/incular-devtools/src/views/geometry.rs`, `tools/incular-devtools/src/views/header.rs`, `tools/incular-devtools/src/views/hover.rs`. Other agents are editing other files at
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
### A050-02 - tools/incular-devtools/src/views/performance.rs:175 (§1, medium)
Evidence:
```
names.get(&id).cloned().unwrap_or_else(|| id.to_string())
```
Suggested fix: Return a Cow so the name is borrowed and only the fallback allocates: `fn widget_label(names: &HashMap<DevWidgetId, String>, id: DevWidgetId) -> Cow<'_, str> { names.get(&id).map_or_else(|| Cow::Owned(id.to_string()), |name| Cow::Borrowed(name.as_str())) }`. Callers at lines 806 (flamegraph, formatted immediately) and 850 (ranked_table, passed to ui_text, which accepts Into<String>) still compile. Requires `use std::borrow::Cow;`.

### A050-01 - tools/incular-devtools/src/views/performance.rs:1045 (§2, medium)
Evidence:
```
deep.push(flamegraph(flame_boxes, widget_names.clone()));
```
Suggested fix: Build the ranked table before the flamegraph and move the map into flamegraph, so the whole HashMap is no longer cloned on every build. In the else branch: `let table = ranked_table(ranked, &widget_names, &bridge, &shared, &tick);` then `deep.push(flamegraph(flame_boxes, widget_names));`, then the existing gap/range-heading pushes, then `deep.push(table);` in place of the later ranked_table call (line 1058). widget_names is not used after this in any branch.

### A053-01 - tools/incular-devtools/src/views/rendering.rs:348 (§2, low)
Evidence:
```
let mut rows: Vec<Widget> = vec![
        Padding::new(
            EdgeInsets::symmetric(12., 0.),
```
Suggested fix: Keep the vec! for the three header widgets, then reserve the row capacity before the loop at line 362: insert `rows.reserve(entries.len());` immediately after the `];` that closes the vec! (line 361), before `for entry in entries {`.

### A053-02 - tools/incular-devtools/src/views/rendering.rs:461 (§2, low)
Evidence:
```
let mut body = Vec::new();
    if !data.supported {
        body.push(newer_target_note("Rendering diagnostics", data.protocol));
```
Suggested fix: Use `let mut body = Vec::with_capacity(5);` (the supported path always pushes exactly five widgets: panel, gap, cards or waiting panel, gap, section).

### A055-01 - tools/incular-devtools/src/views/shell.rs:482 (§2, medium)
Evidence:
```
        format!("{:?}", data.connection),
```
Suggested fix: Add `Hash` to the derive of `ConnectionState` in tools/incular-devtools/src/inspector.rs:46 (it is a fieldless Copy enum) and hash the value directly in `sidebar`: replace the line with `data.connection,`. This removes a String allocation on every sidebar layout pass.

### A055-02 - tools/incular-devtools/src/views/shell.rs:555 (§2, low)
Evidence:
```
    let mut nav = Vec::new();
    for (group, views) in ToolView::GROUPS {
        nav.push(
```
Suggested fix: Size the vector up front, since it holds one header per group and one item per view: `let mut nav = Vec::with_capacity(ToolView::GROUPS.len() + ToolView::ALL.len());`

### A035-04 - crates/incular-devtools/src/commands.rs:51 (§2, low)
Evidence:
```
match serde_json::to_vec(&result) {
Ok(payload) => match crate::BytePermit::try_acquire(
&self.response_payload_bytes,
payload.len(),
```
Suggested fix: Only the byte length is used. Serialize through a counting `std::io::Write` (`serde_json::to_writer`) to get the length without allocating a Vec, keeping the same budget check and the same serialization-failure branch.

### A035-05 - crates/incular-devtools/src/discovery.rs:161 (§4, low)
Evidence:
```
out.sort_by_key(|(record, _)| record.started_unix_ms);
```
Suggested fix: Use `out.sort_unstable_by_key(|(record, _)| record.started_unix_ms);`. The key is a primitive u64 and equal-key order has no meaning, so the stable sort's extra guarantee is unused.

### A040-01 - tools/incular-devtools/src/performance.rs:102 (§2, medium)
Evidence:
```
let mut visited = HashSet::new();
while let Some(parent_index) = parent {
    if !visited.insert(parent_index) {
```
Suggested fix: Hoist the set out of the per-event closure so its table is reused: before `selected.into_iter()` add `let mut visited = HashSet::new();`, then replace the in-closure `let mut visited = HashSet::new();` with `visited.clear();` at the start of the closure body. The closure becomes FnMut, which `map` accepts. Cycle detection and depth results are unchanged.

### A046-05 - tools/incular-devtools/src/views/geometry.rs:31 (§2, low)
Evidence:
```
Align::new(Alignment::CENTER, mono(padding[1].clone(), 11., primary())).into(),
Widget::from(SizedBox::new().width(38.).child(mono(
    padding[0].clone(),
```
Suggested fix: Each padding string is used exactly once, so move them out of the array instead of cloning: `let [left, top, right, bottom] = padding;` (the array order is left, top, right, bottom, as the existing indices show), then use `top`, `left`, `right`, `bottom` in place of `padding[1].clone()`, `padding[0].clone()`, `padding[2].clone()` and `padding[3].clone()`.

### A046-06 - tools/incular-devtools/src/views/header.rs:87 (§2, low)
Evidence:
```
Text::new(label.clone())
    .style(
```
Suggested fix: `label` is only needed twice: once for the Text and once for the button name. Format the button name first and move `label` into the Text: `let name = format!("Live frame rate: {label}");` before building `content`, then `Text::new(label)` and `RawMaterialButton::new(name)`. This removes one String allocation per rebuild.

### A046-07 - tools/incular-devtools/src/views/hover.rs:82 (§8, low)
Evidence:
```
enabled: Option<bool>,
) -> bool {
let start = enabled.unwrap_or(!state.select_mode);
```
Suggested fix: `Option<bool>` encodes three intents (start, stop, toggle) and the doc comment is the only place that explains it. Replace it with a small enum, for example `pub(crate) enum PickRequest { Start, Stop, Toggle }`, and map `Start`/`Stop`/`Toggle` at the call sites. Keep `pick`'s body matching on that enum.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
