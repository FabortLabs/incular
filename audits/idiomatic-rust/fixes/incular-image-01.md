# Fix brief incular-image-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-image/src/lib.rs`. Other agents are editing other files at
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
### A069-01 - crates/incular-image/src/lib.rs:70 (§1, low)
Evidence:
```
self.device_pixel_ratio = if ratio.is_finite() && ratio > 0. {
    ratio
} else {
```
Suggested fix: Add one private helper and call it from all three sites (device_pixel_ratio builder line 70, normalized() line 92, variant_path line 172):
fn finite_positive_or_one(value: f64) -> f64 {
    if value.is_finite() && value > 0. { value } else { 1.0 }
}
Then: self.device_pixel_ratio = finite_positive_or_one(ratio); and in normalized(): self.device_pixel_ratio = finite_positive_or_one(self.device_pixel_ratio); and in variant_path: let value = finite_positive_or_one(device_pixel_ratio);


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
