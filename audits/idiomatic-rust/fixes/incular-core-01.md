# Fix brief incular-core-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-core/src/geometry.rs`, `crates/incular-core/src/restoration.rs`. Other agents are editing other files at
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
### U025-03 - crates/incular-core/src/geometry.rs:600 (§3, low)
Evidence:
```
for (index, value) in rotation.iter_mut().enumerate() {
```
Suggested fix: Zip the three sequences instead of indexing: `for (value, (&begin, &end)) in rotation.iter_mut().zip(begin_rotation.iter().zip(end_rotation.iter())) { *value = mix(begin, end); }`.

### U026-02 - crates/incular-core/src/restoration.rs:229 (§2, medium)
Evidence:
```
        let mut path = self.path.clone();
        path.push(key.clone());
```
Suggested fix: Allocate the final length once: `let mut path = Vec::with_capacity(self.path.len() + 1); path.extend_from_slice(&self.path); path.push(key.clone()); path`. `self.path.clone()` returns a Vec with capacity == len, so the following `push` reallocates on every get/set/remove. The same pattern appears in `child_unchecked` (`let mut path = self.path.clone(); path.push(key);`), so apply the same change there.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
