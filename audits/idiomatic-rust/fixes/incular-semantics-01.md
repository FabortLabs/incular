# Fix brief incular-semantics-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-semantics/src/lib.rs`. Other agents are editing other files at
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
### A156-02 - crates/incular-semantics/src/lib.rs:562 (§1, low)
Evidence:
```
fn truncate(value: Option<&String>, max_chars: usize) -> Option<String> {
 let value = value?;
 if value.chars().count() <= max_chars {
```
Suggested fix: Take the borrowed text form: `fn truncate(value: Option<&str>, max_chars: usize) -> Option<String>` and call it with `node.label.as_deref()` / `node.value.as_deref()` at the two call sites (lines 611-612). The body is unchanged apart from the `value.clone()` becoming `value.to_owned()`.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
