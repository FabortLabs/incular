# Fix brief incular-macos-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-macos/src/application_shell.rs`, `crates/incular-macos/src/environment.rs`, `crates/incular-macos/src/platform_menus.rs`. Other agents are editing other files at
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
### A074-01 - crates/incular-macos/src/application_shell.rs:268 (§16, high)
Evidence:
```
let center = unsafe { UNUserNotificationCenter::currentNotificationCenter() };
let category_identifier = match self.install_category(id, &presentation, &center) {
```
Suggested fix: Add a `// SAFETY:` comment directly above each unsafe block that lacks one, stating the invariant (main-thread AppKit/UserNotifications use, process-global center, retained objects alive for the synchronous call). Blocks lacking one: lines 268, 274, 281, 367, 390, 400, 436, 437, 461, 490. Existing shared comments (e.g. lines 422-425) do not cover them.

### A074-03 - crates/incular-macos/src/application_shell.rs:320 (§12, medium)
Evidence:
```
let mut pending = pending_authorization
.lock()
.expect("macOS pending notification authorization lock");
```
Suggested fix: Decide the outcome while holding the lock, drop the guard, then invoke the completion callback. For the stale branch: `let stale = pending.get(&id).copied() != Some(request_id); if stale { drop(pending); authorization_complete(NativeApplicationShellCompletion { request_id, result: Err(ApplicationShellError::StaleResource) }); return; }`. Apply the same drop-before-callback to the completion call near line 360-364 (`drop(pending);` should precede `authorization_complete(...)`). The completion sink is a callback into the runtime and must not run under the pending-authorization mutex.

### A074-02 - crates/incular-macos/src/environment.rs:125 (§16, high)
Evidence:
```
let observer = unsafe {
center.addObserverForName_object_queue_usingBlock(
```
Suggested fix: Add a `// SAFETY:` comment directly above each unsafe block lacking one: lines 115 (extern static read), 125, 143 (extern static read), 153, 189, 201, 219. The comment at lines 106-108 and 113-114 is not labelled SAFETY and does not cover these blocks individually. Example: `// SAFETY: watcher installation runs on the AppKit main thread; NSNotificationCenter copies the block and the observer token is stored in self.observers.`

### A075-01 - crates/incular-macos/src/platform_menus.rs:382 (§16, high)
Evidence:
```
    unsafe {
        item.setEnabled(node.enabled);
```
Suggested fix: Add a SAFETY comment directly above each unsafe block: at line 382 (build_menu_item, setEnabled/setToolTip), line 393 (setTag/setTarget), line 450 (update_menu_nodes, setTitle/setEnabled/setToolTip) and line 465 (setTag/setKeyEquivalent), e.g. `// SAFETY: main-thread AppKit call on a menu item owned by this build/update pass.`


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
