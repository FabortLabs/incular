# Fix brief incular-devtools-06

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `tools/incular-devtools/src/views/overview.rs`, `tools/incular-devtools/src/inspector.rs`, `tools/incular-devtools/src/views.rs`, `tools/incular-devtools/src/views/application.rs`, `tools/incular-devtools/src/views/overlays.rs`. Other agents are editing other files at
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
### A049-01 - tools/incular-devtools/src/views/overview.rs:199 (§2, medium)
Evidence:
```
let memory = state.memory.clone();
```
Suggested fix: Before drop(state), keep only the value the view reads: let memory_rss_mb = state.memory.as_ref().map(|snapshot| snapshot.counts.rss_mb); Then remove the clone at line 199 and, in the Process memory card (lines 306-316), use memory_rss_mb: map_or_else(|| "—".into(), |mb| if mb == 0 { "—".into() } else { format!("{mb} MB") }) and replace memory.is_some() at line 316 with memory_rss_mb.is_some(). Avoids cloning the whole MemorySnapshot (label String and buckets Vec) on every overview build.

### A049-02 - tools/incular-devtools/src/views/overview.rs:359 (§2, medium)
Evidence:
```
detail: error
.clone()
.unwrap_or_else(|| "Start an app with --devtools to inspect it".into()),
```
Suggested fix: Drop the redundant clone: detail: error.unwrap_or_else(|| "Start an app with --devtools to inspect it".into()), . error (line 217) is not used after this block, so it can be moved here.

### A049-03 - tools/incular-devtools/src/views/overview.rs:624 (§5, low)
Evidence:
```
let color = match entry.level.to_ascii_lowercase().as_str() {
"error" | "fatal" => danger(),
"warn" | "warning" => warning(),
```
Suggested fix: Avoid allocating a lowercased copy per message: let level = entry.level.as_str(); let color = if level.eq_ignore_ascii_case("error") || level.eq_ignore_ascii_case("fatal") { danger() } else if level.eq_ignore_ascii_case("warn") || level.eq_ignore_ascii_case("warning") { warning() } else if level.eq_ignore_ascii_case("debug") || level.eq_ignore_ascii_case("trace") { text_muted() } else { super::shared::info() };

### A038-01 - tools/incular-devtools/src/inspector.rs:188 (§5, medium)
Evidence:
```
Self::Errors => matches!(level.to_ascii_lowercase().as_str(), "error" | "fatal"),
Self::Warnings => matches!(level.to_ascii_lowercase().as_str(), "warn" | "warning"),
Self::Debug => matches!(level.to_ascii_lowercase().as_str(), "debug" | "trace"),
```
Suggested fix: Compare case-insensitively without allocating, as Self::Info already does:
Self::Errors => level.eq_ignore_ascii_case("error") || level.eq_ignore_ascii_case("fatal"),
Self::Warnings => level.eq_ignore_ascii_case("warn") || level.eq_ignore_ascii_case("warning"),
Self::Debug => level.eq_ignore_ascii_case("debug") || level.eq_ignore_ascii_case("trace"),

### A039-01 - tools/incular-devtools/src/inspector.rs:1071 (§2, medium)
Evidence:
```
let contains = |value: &str| value.to_lowercase().contains(*term);
match term.split_once(':') {
_ => { contains(&node.type_name) || node.label.as_deref().is_some_and(contains) ...
```
Suggested fix: Lowercase each node field once per node, outside the per-term closure, and reuse it for every term: before `let matches = terms.iter().all(|term| {`, add `let type_lower = node.type_name.to_lowercase(); let label_lower = node.label.as_deref().map(str::to_lowercase); let key_lower = node.key.as_deref().map(str::to_lowercase); let id_text = node.id.to_string().to_lowercase();`, then in the arms use `type_lower.contains(value)`, `label_lower.as_deref().is_some_and(|text| text.contains(value))`, `key_lower.as_deref().is_some_and(|key| key.contains(value))`, `id_text.contains(value)`, and in the `_` arm `type_lower.contains(*term) || label_lower.as_deref().is_some_and(|t| t.contains(*term)) || key_lower.as_deref().is_some_and(|k| k.contains(*term)) || id_text.contains(*term)`. Remove the `contains` closure. Only runs when a search query is active; allocations drop from terms x fields per node to fields per node.

### A043-01 - tools/incular-devtools/src/views.rs:233 (§2, medium)
Evidence:
```
let application_frames = if active_view == ToolView::Application {
state.frames.iter().cloned().collect()
```
Suggested fix: let mut frames = Vec::with_capacity(state.frames.len());
frames.extend(state.frames.iter().cloned());
frames
(length is known up front, so the Vec is sized once instead of growing while cloning every retained frame on each root rebuild; if the copy itself is the cost, consider sharing the frames behind an Arc in the model instead of cloning them)

### A043-02 - tools/incular-devtools/src/views.rs:474 (§2, medium)
Evidence:
```
windows: snapshot.windows.clone(),
details: snapshot.details.clone(),
selected_details: snapshot.selected_details.clone(),
```
Suggested fix: Move the fields out of `snapshot` instead of cloning them: `windows: snapshot.windows,`, `details: snapshot.details,`, `selected_details: snapshot.selected_details,`, and likewise `signals: snapshot.signals,`, `selected_signal: snapshot.selected_signal,`, `signal_subscribers: snapshot.signal_subscribers,` (lines 474-483). `snapshot` is a per-build local whose fields are not read after the match, so the clones only copy data that is dropped. The same applies to `snapshot.diagnostics.clone()` and `snapshot.work_counts.clone()` in the Rendering arm (lines 449-450).

### A044-01 - tools/incular-devtools/src/views/application.rs:39 (§1, medium)
Evidence:
```
    windows: Vec<WindowSummary>,
    frames: Vec<FrameRecordEvent>,
```
Suggested fix: Take the frames by reference: change the parameter to `frames: &[FrameRecordEvent],` (the body only calls `frames.iter()`). Then stop cloning the whole frame list in the caller: views.rs:232-236 builds `application_frames` with `state.frames.iter().cloned().collect()` on every snapshot rebuild while the Application view is active. Replace that field with the per-window counts computed under the lock (for example a `HashMap<WindowId, usize>` or a small Vec built from `state.frames` without cloning), and have build_application take those counts instead of frames. The current call passes the owned Vec, which forces the clone on every rebuild.

### A044-02 - tools/incular-devtools/src/views/application.rs:104 (§2, low)
Evidence:
```
Expanded::new(line_text(window.title.clone(), 13.5, text_primary())).into(),
badge(format!("{observed} frames"), text_muted()),
```
Suggested fix: Move the title instead of cloning it: `Expanded::new(line_text(window.title, 13.5, text_primary())).into(),`. `window` is the by-value loop binding from `for window in windows`, and `window.title` is not used again in the loop body (the later uses are `id`, `logical_size` and `scale_factor`, which are not affected by the partial move).

### A049-04 - tools/incular-devtools/src/views/overlays.rs:170 (§8, low)
Evidence:
```
if group.title == "Performance" && state.protocol < PERFORMANCE_OVERLAY_PROTOCOL {
```
Suggested fix: Make the protocol gate data on the group instead of matching its display title: add a field min_protocol: u32 to OverlayGroup (0 for ungated groups), set min_protocol: PERFORMANCE_OVERLAY_PROTOCOL on the Performance entry in GROUPS, and write: if state.protocol < group.min_protocol { ... }. Renaming the title then cannot silently drop the gate.

### A049-05 - tools/incular-devtools/src/views/overlays.rs:178 (§2, low)
Evidence:
```
let mut rows = Vec::new();
for &(option, label) in group.options {
```
Suggested fix: let mut rows = Vec::with_capacity(group.options.len());


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
