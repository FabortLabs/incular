# Fix brief incular-devtools-08

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `tools/incular-devtools/src/views/network.rs`, `tools/incular-devtools/src/views/pump.rs`, `tools/incular-devtools/src/views/shared.rs`, `tools/incular-devtools/src/views/tree.rs`. Other agents are editing other files at
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
### A048-01 - tools/incular-devtools/src/views/network.rs:206 (§2, medium)
Evidence:
```
let entries = shared
        .lock()
        .map(|state| state.activity.entries().rev().cloned().collect::<Vec<_>>())
```
Suggested fix: Clone only what the view needs, inside the lock scope. Compute total, errors, pending, latency, p95, sent and received by borrowing `state.activity.entries()`, then build `visible` as `Vec<RequestActivity>` from borrowed survivors (filter on the borrow, `.cloned()` only the survivors), and clone only the selected entry for `detail`. Drop the whole-collection `Vec` clone and the second clone of survivors at lines 237-238 (`.cloned()` on `visible`), which clones the same entries twice per build.

### A052-03 - tools/incular-devtools/src/views/pump.rs:133 (§8, low)
Evidence:
```
            let (slot, interval) = if shows_telemetry(self.view.get()) {
                (1, TELEMETRY_INTERVAL)
            } else {
```
Suggested fix: Replace the magic slot indices 0/1/2 (`last: [Option<Instant>; 3]`, `slot == 1`, `slot == 2`) with a small enum such as `enum Pace { Content, Telemetry, Header }` that indexes `last` through `as usize` (or named `const` indices). The `if slot == 1 { refresh(&self.header) }` branch at lines 139-147 would then read as an explicit match arm that refreshes both the telemetry and header signals.

### A054-02 - tools/incular-devtools/src/views/shared.rs:624 (§8, low)
Evidence:
```
pub(crate) fn segmented_control(
    options: impl IntoIterator<Item = (String, bool, Box<dyn Fn()>)>,
```
Suggested fix: pub(crate) fn segmented_control(options: impl IntoIterator<Item = TabOption>) -> Widget {
    tab_list(options)
}
// Reuse the existing TabOption alias (line 573) instead of restating the tuple type.

### A056-01 - tools/incular-devtools/src/views/tree.rs:177 (§2, low)
Evidence:
```
    } else {
        path.clone()
    };
```
Suggested fix: Borrow instead of cloning the whole path when it is short, and size the truncated buffer up front:
    let visible_path: Cow<[DevWidgetId]> = if path.len() > 5 {
        let mut short = Vec::with_capacity(5);
        short.push(path[0]);
        short.extend_from_slice(&path[path.len() - 4..]);
        Cow::Owned(short)
    } else {
        Cow::Borrowed(&path)
    };
(add `use std::borrow::Cow;`; the loop `visible_path.iter().copied().enumerate()` is unchanged)


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
