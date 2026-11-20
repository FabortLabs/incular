# Fix brief incular-desktop-03

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-desktop/src/single_instance.rs`, `crates/incular-desktop/src/winit_adapter.rs`. Other agents are editing other files at
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
### A031-01 - crates/incular-desktop/src/single_instance.rs:574 (§2, low)
Evidence:
```
Self::Unix(bytes) => {
    #[cfg(unix)]
    Ok(OsString::from_vec(bytes.clone()))
```
Suggested fix: Add a consuming `fn into_os_string(self) -> Result<OsString, SingleInstanceError>` whose Unix arm is `Self::Unix(bytes) => Ok(OsString::from_vec(bytes))`, and call it from `into_activation` via `into_iter()` instead of `iter()` + `to_os_string`. Keep the borrowing `to_os_string(&self)` for `connect_endpoint`, which only holds `&Endpoint`.

### A330-02 - crates/incular-desktop/src/winit_adapter.rs:411 (§2, low)
Evidence:
```
            format!("{code:?}").parse().unwrap_or(Code::Unidentified)
```
Suggested fix: Replace the Debug-string round trip with a direct `match` over `winit::keyboard::KeyCode` that returns the matching `Code` variant, so no String is allocated per key event. Apply the same change to `logical_key`, which does `format!("{named:?}").parse::<NamedKey>()` for each named key.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
