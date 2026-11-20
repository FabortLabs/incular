# Fix brief incular-text-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-text/src/engine/inline.rs`, `crates/incular-text/src/engine/document.rs`. Other agents are editing other files at
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
### A160-01 - crates/incular-text/src/engine/inline.rs:174 (§2, high)
Evidence:
```
                .flat_map(|line| line.runs().collect::<Vec<_>>())
                .find(|run| run.text_range().contains(&placeholder.index))
```
Suggested fix: Replace the layout.lines().flat_map(..).find(..).map(..)? chain in center_middle_boxes (inline.rs:172-176) with a nested search that allocates nothing and stops at the first hit:
            let (ascent, descent) = layout
                .lines()
                .find_map(|line| {
                    line.runs()
                        .find(|run| run.text_range().contains(&placeholder.index))
                        .map(|run| (run.font_metrics().ascent, run.font_metrics().descent))
                })?;

### A160-02 - crates/incular-text/src/engine/document.rs:136 (§2, low)
Evidence:
```
            (0..self.line_count)
                .map(|_| OnceLock::new())
                .collect::<Vec<_>>()
```
Suggested fix: Collect straight into the Arc<[_]> target instead of a Vec that is then copied by .into(); the Map over a Range is TrustedLen, so this is a single allocation:
        let lines = self.line_carets.get_or_init(|| {
            (0..self.line_count).map(|_| OnceLock::new()).collect()
        });


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
