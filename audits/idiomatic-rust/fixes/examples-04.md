# Fix brief examples-04

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `examples/studio/views/bottom_panel.rs`, `examples/studio/views/inspector.rs`, `examples/studio/views/project_tree.rs`, `examples/studio/views/status_bar.rs`. Other agents are editing other files at
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
### A326-07 - examples/studio/views/bottom_panel.rs:106 (§2, medium)
Evidence:
```
let docs = state.editor.documents.get();

let search_row_items: Vec<Widget> = vec![
```
Suggested fix: Do not clone the document list on every build of the search view. Capture the signal instead (`let documents_sig = state.editor.documents.clone();`) and call `let docs = documents_sig.get();` inside the `on_submit` closure, so the Vec<DocumentTab> (with its Strings) is cloned only when a search runs.

### A327-04 - examples/studio/views/inspector.rs:25 (§2, low)
Evidence:
```
let text = d.controller.text();
let line_count = text.lines().count();
let char_count = text.chars().count();
```
Suggested fix: `text()` returns an owned copy of the whole buffer (crates/incular-text/src/editing.rs:415) and the result is only read. Take the three counts from a borrowed view instead, e.g. a `with_text(|t| (t.lines().count(), t.chars().count(), t.len()))` accessor added to TextEditingController, so the buffer is not cloned per build.

### A327-05 - examples/studio/views/project_tree.rs:29 (§2, medium)
Evidence:
```
let matches_filter = filter.is_empty() || node.name.to_lowercase().contains(&filter);
```
Suggested fix: Allocates a lowercase copy of every node name on every build while a filter is active, and the loop visits all nodes. Store a pre-lowercased name on `TreeNode` when the tree is built (for example `name_lower: String`) and test `node.name_lower.contains(&filter)`. Alternatively, reuse one scratch String across nodes with `clear()` and `push_str`, but the stored field avoids the work entirely.

### A328-01 - examples/studio/views/status_bar.rs:14 (§6, low)
Evidence:
```
let (line, col) = match doc {
    Some(d) => (d.cursor_line.get(), d.cursor_col.get()),
    None => (1, 1),
```
Suggested fix: let (line, col) = doc.map_or((1, 1), |d| (d.cursor_line.get(), d.cursor_col.get()));


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
