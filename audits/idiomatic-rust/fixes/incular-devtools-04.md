# Fix brief incular-devtools-04

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `tools/incular-devtools/src/views/console.rs`, `tools/incular-devtools/src/views/widget_inspector.rs`, `crates/incular-devtools/src/lib.rs`. Other agents are editing other files at
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
### A046-02 - tools/incular-devtools/src/views/console.rs:31 (§5, low)
Evidence:
```
match level.to_ascii_lowercase().as_str() {
let lower = entry.level.to_ascii_lowercase();
let (stripe, wash) = match lower.as_str() {
```
Suggested fix: `row` lowercases the same level twice per entry (once via `level_color`, again at line 86). Lowercase once in `row` and pass the borrowed form: `let lower = entry.level.to_ascii_lowercase(); let color = level_color(&lower);` and change `level_color(level: &str)` to match on `level` directly, or compare with `eq_ignore_ascii_case` for the arms.

### A046-04 - tools/incular-devtools/src/views/console.rs:41 (§2, low)
Evidence:
```
let mut grouped: Vec<(ConsoleEntry, usize)> = Vec::new();
for entry in entries {
```
Suggested fix: The output holds at most `entries.len()` groups, so reserve up front: `let mut grouped: Vec<(ConsoleEntry, usize)> = Vec::with_capacity(entries.len());` (the `entries` parameter is still available before the loop consumes it).

### A046-03 - tools/incular-devtools/src/views/console.rs:70 (§5, low)
Evidence:
```
.collect::<Vec<_>>()
.join("\n")
```
Suggested fix: Build one String instead of a format! per entry, a Vec, and a join: `let mut text = String::new(); for (index, entry) in entries.iter().enumerate() { if index > 0 { text.push('\n'); } let _ = write!(text, "[{}] {} {}: {}", session_time(entry.time_ms), entry.level.to_uppercase(), entry.target, entry.message); } text`. Add `use std::fmt::Write;`.

### A046-01 - tools/incular-devtools/src/views/console.rs:225 (§2, medium)
Evidence:
```
let entries = entries.clone();
button(
    "Copy",
```
Suggested fix: Share the visible rows instead of deep-cloning them on every build: convert once with `let entries: Rc<[ConsoleEntry]> = entries.into();` and capture `Rc::clone(&entries)` in the copy closure (`console_text(&copy_entries)`). Let `group_repeats` take `&[ConsoleEntry]` (or iterate `entries.iter()`) so the Vec is no longer moved, and clone only the displayed fields in `row`.

### A057-04 - tools/incular-devtools/src/views/widget_inspector.rs:319 (§2, low)
Evidence:
```
let mut tabs: Vec<super::shared::TabOption> = Vec::new();
for (section, label) in [
    (InspectorSection::Properties, "Properties"),
```
Suggested fix: let mut tabs: Vec<super::shared::TabOption> = Vec::with_capacity(6);

### A057-03 - tools/incular-devtools/src/views/widget_inspector.rs:388 (§2, low)
Evidence:
```
state
    .property_error
    .clone()
```
Suggested fix: Filter by reference and clone only the survivor: `state.property_error.as_ref().filter(|(id, _, _)| *id == details.id).cloned()`

### A057-02 - tools/incular-devtools/src/views/widget_inspector.rs:394 (§2, medium)
Evidence:
```
property.name.to_lowercase().contains(&filter)
|| debug_value(&property.value)
    .to_lowercase()
```
Suggested fix: Short-circuit the empty filter before allocating, so the default (no filter) case allocates nothing per property: change the filter closure body to `filter.is_empty() || property.name.to_lowercase().contains(&filter) || debug_value(&property.value).to_lowercase().contains(&filter)`. Matching is unchanged because an empty needle matches every property.

### A057-01 - tools/incular-devtools/src/views/widget_inspector.rs:757 (§2, medium)
Evidence:
```
let closed = collapsed.get().contains(&title);
```
Suggested fix: let closed = collapsed.with(|groups| groups.contains(&title));

### A035-03 - crates/incular-devtools/src/lib.rs:92 (§2, low)
Evidence:
```
let Some(bytes) = serde_json::to_vec(&event).ok().map(|payload| payload.len()) else {
```
Suggested fix: Measure the serialized size without allocating: serialize with `serde_json::to_writer(&mut counter, &event)` into a small private counting `std::io::Write` implementor (sums `buf.len()`), keeping the `Err` branch as is. The Vec is only used for its length.

### A035-02 - crates/incular-devtools/src/lib.rs:258 (§2, low)
Evidence:
```
let app_name = config.app_name.clone();
let _ = config;
```
Suggested fix: Move the field instead of cloning and delete the no-op: `let app_name = config.app_name;` (remove `let _ = config;`), then pass `app_name` by move at line 263 (`app_name,`) without `.clone()`. SessionConfig has no Drop impl, so the partial move is allowed.

### A035-01 - crates/incular-devtools/src/lib.rs:336 (§23, medium)
Evidence:
```
std::time::SystemTime::now()
.duration_since(std::time::UNIX_EPOCH)
```
Suggested fix: Use the wasm-safe clock: `use incular_core::time::{SystemTime, UNIX_EPOCH};` and call `SystemTime::now()` there (web_time re-export). Also `std::process::id()` at lines 263 and 350 is forbidden on wasm32 (repo platform facts) if this code is reachable there. Adding the incular-core dependency needs a matching update to specs/architecture.json allow-list. Reachability: spawn() is called from incular-desktop under the `devtools` feature, which is not gated off for wasm32.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
