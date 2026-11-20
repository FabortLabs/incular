# Fix brief incular-runtime-05

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-runtime/src/simulation.rs`, `crates/incular-runtime/src/undo.rs`, `crates/incular-runtime/src/window_state.rs`. Other agents are editing other files at
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
### A145-01 - crates/incular-runtime/src/simulation.rs:261 (§12, medium)
Evidence:
```
if let Some(wake) = self.wake.lock().expect("simulation wake mutex").as_ref() {
wake.wake();
```
Suggested fix: let wake = self.wake.lock().expect("simulation wake mutex").clone();
if let Some(wake) = wake {
    wake.wake();
}

### A145-02 - crates/incular-runtime/src/simulation.rs:302 (§2, medium)
Evidence:
```
for reply in pending {
let _ = reply.send(result.clone());
```
Suggested fix: let Some(mut pending) = waiters.remove(&window_id) else { return };
let Some(last) = pending.pop() else { return };
for reply in pending {
    let _ = reply.send(result.clone());
}
let _ = last.send(result);

### A145-03 - crates/incular-runtime/src/simulation.rs:364 (§6, low)
Evidence:
```
let result = capture
.map(|result| result.map_err(SimulationError::CaptureUnavailable))
.unwrap_or_else(|| {
```
Suggested fix: let result = capture.map_or_else(
    || Err(SimulationError::CaptureUnavailable("the renderer did not return a capture".into())),
    |result| result.map_err(SimulationError::CaptureUnavailable),
);

### A145-04 - crates/incular-runtime/src/simulation.rs:743 (§10, low)
Evidence:
```
.pressed
.iter()
.copied()
```
Suggested fix: Iterate in a deterministic order: if Code implements Ord, make KeyboardState::pressed a BTreeSet<Code> (and modifiers_for_pressed take &BTreeSet<Code>); otherwise sort the collected Vec before the key-up loop. Verify Code's derives first.

### A148-04 - crates/incular-runtime/src/undo.rs:131 (§10, low)
Evidence:
```
let remove = entries.len() - max_entries;
entries.drain(..remove);
```
Suggested fix: Once the history is full, every committed edit removes the oldest step from the front of a Vec, which shifts the remaining entries. Store `undo` and `redo` as `VecDeque<Step>`, then use `pop_front()` while `len() > max_entries`. `pop_back`/`push_back` replace the current `pop`/`push`, and the rest of the logic stays the same.

### A148-02 - crates/incular-runtime/src/undo.rs:213 (§23/repo wasm rule, low)
Evidence:
```
if !std::thread::panicking() {
    self.inner.sync_state();
```
Suggested fix: The checklist forbids `std::thread` in code reachable on wasm32, and this Drop impl is in shared runtime code. Replace the unwind check with an explicit flag. Add `completed: bool` to `RestorePrevious`, set `guard.completed = true` right after `self.editor.set_value(value)`, and in Drop call `self.inner.sync_state()` only `if self.completed`. Behavior is the same: state is not synced while unwinding.

### A148-01 - crates/incular-runtime/src/undo.rs:260 (§2, medium)
Evidence:
```
if let Some(inner) = weak.upgrade() {
    inner.record(inner.editor.value());
```
Suggested fix: The listener runs on every editor notification, including caret and selection moves, and `editor.value()` clones the whole text each time. Read the value borrowed with `inner.editor.with_value(|value| ...)` (incular-text editing.rs:409) and have `record` take `&TextEditingValue`. Compare text on the borrowed value. Clone `value.text` only when the text actually changed. On selection-only change, copy the fields: `history.current.selection = value.selection; history.current.composing = value.composing;`. Keep `sync_state()` outside the `with_value` closure so a listener that edits the editor cannot hit a RefCell borrow conflict. Check that `with_value` is re-entrancy safe before relying on it.

### A148-03 - crates/incular-runtime/src/undo.rs:268 (§2, low)
Evidence:
```
history: RefCell::new(History {
    current: initial.clone(),
```
Suggested fix: `initial` is not used after this point and `Rc::new_cyclic` takes an FnOnce, so move it instead of cloning: `current: initial,`.

### A150-03 - crates/incular-runtime/src/window_state.rs:324 (§18, low)
Evidence:
```
#[cfg_attr(not(feature = "devtools"), allow(dead_code))]
pub(crate) fn get(&self, id: WindowId) -> Option<&WindowRecord> {
```
Suggested fix: Confirm whether `get` is still dead without the devtools feature. It is called by WindowManager::handle (L608), so the allow is probably stale; delete the attribute. If it is genuinely dead in some configuration, use `#[cfg_attr(not(feature = "devtools"), expect(dead_code, reason = "only the devtools diagnostics read this"))]` instead.

### A150-02 - crates/incular-runtime/src/window_state.rs:493 (§2, medium)
Evidence:
```
restoration_scope.clone(),
```
Suggested fix: The retained root builder closure (L486-498) clones restoration_scope on every root rebuild. RestorationScope::clone allocates a Vec<RestorationKey> (each key owns a String) for any non-empty path. Hoist the value once before the closure (`let restoration_scope = restoration_scope.map(Rc::new);`) and change BuildContext::new to take the shared handle (Option<Rc<RestorationScope>>, or &RestorationScope), so each rebuild only bumps an Rc count. Keep the initial clone at L454 consistent with the same change.

### A150-04 - crates/incular-runtime/src/window_state.rs:688 (§2, low)
Evidence:
```
.windows()
.into_iter()
.find(|saved| saved.restoration_id == metadata.id.as_key().as_str())
```
Suggested fix: RestorationManager::windows() (restoration.rs L699) clones the whole snapshot Vec<RestoredWindow>, including every restoration_id and kind String, and then discards all but one. Add a pub(crate) lookup such as `fn window(&self, restoration_id: &str) -> Option<RestoredWindow>` on RestorationManager that clones only the matching entry, and call it here as `manager.window(metadata.id.as_key().as_str())`.

### A150-05 - crates/incular-runtime/src/window_state.rs:758 (§2, low)
Evidence:
```
let active_ids = active_windows
.iter()
.map(|window| window.restoration_id.clone())
```
Suggested fix: Each active restoration_id String is cloned into a HashSet<String> only to answer membership queries. Avoid the clones and the extra Vec: `let mut windows = manager.windows(); let active_ids: HashSet<&str> = active_windows.iter().map(|window| window.restoration_id.as_str()).collect(); windows.retain(|window| !active_ids.contains(window.restoration_id.as_str())); windows.extend(active_windows);` in the retain_unopened branch. The borrow ends before extend moves active_windows, which is why the clone was a borrow workaround.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
