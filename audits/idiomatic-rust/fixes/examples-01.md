# Fix brief examples-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `examples/issue_tracker/main.rs`, `examples/performance_gallery/main.rs`, `examples/studio/state.rs`. Other agents are editing other files at
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
### A320-01 - examples/issue_tracker/main.rs:103 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
fn ruled(
```
Suggested fix: #[expect(clippy::too_many_arguments, reason = "mirrors the source CSS per-edge separator geometry")]
fn ruled(  // 8 parameters, so the expectation is fulfilled

### A320-02 - examples/issue_tracker/main.rs:248 (§7, low)
Evidence:
```
.map_err(|_| std::io::Error::other("invalid embedded issue dataset"))?;
```
Suggested fix: .map_err(|error| std::io::Error::other(format!("invalid embedded issue dataset: {error:?}")))?;  // keep the decompression error as the message detail

### A320-05 - examples/issue_tracker/main.rs:258 (§8, medium)
Evidence:
```
let filter = Signal::new("All issues".to_owned());
(active_filter == "All issues"
```
Suggested fix: Replace the stringly-typed filter with an enum: `#[derive(Clone, Copy, PartialEq, Eq)] enum Filter { All, Open, Completed }` with `fn label(self) -> &'static str` returning "All issues"/"Open"/"Completed". Use `Signal::new(Filter::All)`, match on `active_filter` at lines 308-310, iterate `[Filter::All, Filter::Open, Filter::Completed]` at line 424 with `Button::new(status.label())`, and `filter.set(status)` at line 456.

### A320-03 - examples/issue_tracker/main.rs:312 (§2, medium)
Evidence:
```
|| format!(
"{} {} {} {}",
.to_lowercase()
```
Suggested fix: Hoist two String buffers before the filter (`let mut haystack = String::new(); let mut lowered = String::new();`, plus `use std::fmt::Write;`) and reuse them inside the filter closure: `haystack.clear(); let _ = write!(haystack, "{} {} {} {}", issue.id, issue.title, issue.project, issue.owner); lowered.clear(); lowered.extend(haystack.chars().flat_map(char::to_lowercase)); lowered.contains(&needle)`. Note: str::to_lowercase handles final sigma differently from per-char lowercasing, so this is a tiny search-semantics change.

### A320-04 - examples/issue_tracker/main.rs:385 (§2, medium)
Evidence:
```
let issue = data[selected_index].clone();
drop(data);
```
Suggested fix: Borrow instead of cloning the whole Issue (8 Strings) on every build: `let issue = &data[selected_index];` and move `drop(data);` (line 397) to after the last use of `issue` at the end of the build block. The issues RefCell is not borrowed mutably during build (the on_changed and on_click borrow_mut calls run later from events), so no runtime conflict. Alternatively copy only the few fields used.

### A323-01 - examples/performance_gallery/main.rs:400 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
pub(crate) fn scenario_view(
```
Suggested fix: #[expect(clippy::too_many_arguments, reason = "each retained workload takes its own state handle")]

### A323-03 - examples/performance_gallery/main.rs:414 (§8, low)
Evidence:
```
match selected {
0 => hundred_thousand_widgets(),
_ => Widget::from(Text::new("unknown")),
```
Suggested fix: Model the workload as `enum Scenario { HundredThousandWidgets, MillionFixedList, ... }` with `const ALL: [Scenario; 15]` and `fn label(self) -> &'static str`, store the selected `Scenario` in the Signal instead of an index into SCENARIOS, and match exhaustively so the `_ => unknown` arm and the parallel SCENARIOS table (`SCENARIOS[selected]`, `SCENARIOS.len()`) go away.

### A323-02 - examples/performance_gallery/main.rs:663 (§2, low)
Evidence:
```
.map(|chunk| Widget::from(Row::new(chunk.to_vec())))
.collect::<Vec<_>>();
```
Suggested fix: Build each row straight from its index range instead of cloning out of a temporary Vec: move the cell constructor into a closure `cell(index)` and write `let rows: Vec<Widget> = (0..200_usize).step_by(10).map(|start| Widget::from(Row::new((start..start + 10).map(cell)))).collect();`. Same pattern at line 713 (many_paths, `chunks(6)`) and line 848 (gesture_stress, `rows.push(Widget::from(Row::new(chunk.to_vec())))`): the `grid`/`cells` Vec is dropped right after, so the per-element clones are avoidable.

### A326-01 - examples/studio/state.rs:99 (§2, medium)
Evidence:
```
pub nodes: Arc<Vec<TreeNode>>,
pub expanded_nodes: Signal<HashSet<String>>,
```
Suggested fix: Change the field to `pub nodes: Arc<[TreeNode]>,` and at line 223 build it with `nodes: Arc::from(nodes),` (one copy at startup only; callers in views/project_tree.rs use len, indexing and Arc::clone, which work unchanged on Arc<[T]>).

### A326-03 - examples/studio/state.rs:256 (§8, low)
Evidence:
```
pub fn to_restoration(&self) -> RestorationData {
```
Suggested fix: Add `#[must_use]` above `pub fn to_restoration`, matching `active_document` and `StudioState::new`.

### A326-02 - examples/studio/state.rs:265 (§8, low)
Evidence:
```
theme_mode: format!("{:?}", self.theme_mode.get()),
```
Suggested fix: Persist the theme through a stable name (e.g. a `ThemeMode::as_str(self) -> &'static str` next to a matching parser used at line 157) instead of relying on Debug output; a renamed variant currently falls back to Dark silently.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
