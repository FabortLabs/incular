# Fix brief incular-runtime-04

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-runtime/src/application_shell.rs`, `crates/incular-runtime/src/environment.rs`, `crates/incular-runtime/src/frame/text_actions.rs`. Other agents are editing other files at
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
### A129-01 - crates/incular-runtime/src/application_shell.rs:413 (§12, low)
Evidence:
```
let state = self.state.borrow_mut();
```
Suggested fix: Use `self.state.borrow()` on this line and on line 438 (NotificationAction arm). Both blocks only read the state, and borrow_mut on a read-only path panics on any re-entrant borrow and misstates intent.

### A129-02 - crates/incular-runtime/src/application_shell.rs:414 (§2, medium)
Evidence:
```
tray_record(&state.trays, id).map(|record| record.menu.clone())
```
Suggested fix: Store `menu: Arc<PlatformMenuCommandModel>` in `TrayRecord` (wrap in `Arc::new` at the create path, line 235, and the update path, line 760). Then replace this clone with `.map(|record| Arc::clone(&record.menu))`. The borrow still ends before `dispatch` runs, which is required because callbacks may re-enter the service. Only the Arc is cloned per tray menu event instead of the whole command model.

### A129-05 - crates/incular-runtime/src/application_shell.rs:489 (§19, low)
Evidence:
```
let pending: Vec<_> = state
```
Suggested fix: The collect exists only to dodge the borrow through the RefMut guard. Use `let state = &mut *self.state.borrow_mut();` and then `state.completions.extend(state.pending.drain().map(|(request_id, _)| NativeApplicationShellCompletion { request_id, result: Err(ApplicationShellError::ApplicationStopped) }));`. The two fields are disjoint, so this compiles without the Vec.

### A129-03 - crates/incular-runtime/src/application_shell.rs:615 (§2, medium)
Evidence:
```
state.pending.insert(request_id, operation.clone());
```
Suggested fix: `pending` only needs the resource that `complete()` later uses via `created_resource`. Change the field to `RequestRegistry<ApplicationShellRequestId, Option<ShellResource>>`, compute `let resource = created_resource(&operation);` in `enqueue` and call `state.pending.insert(request_id, resource);`. Update `complete()` to match `Ok(resource)` and use `if let Some(resource) = resource`, and update the `stop()` drain pattern to ignore the value. This removes a deep clone of the full operation (including menu snapshots) on every enqueue.

### A131-02 - crates/incular-runtime/src/environment.rs:378 (§2, low)
Evidence:
```
self.nodes.remove(&node_id);
self.queued_nodes.remove(&node_id);
self.node_order.retain(|queued| *queued != node_id);
```
Suggested fix: Run the O(queue) retain only when the node was actually queued, since queued_nodes and node_order are kept in sync:
self.nodes.remove(&node_id);
if self.queued_nodes.remove(&node_id) {
    self.node_order.retain(|queued| *queued != node_id);
}

### A131-01 - crates/incular-runtime/src/environment.rs:390 (§3, low)
Evidence:
```
let mut ids: HashSet<_> = self.dependencies.keys().copied().collect();
ids.extend(self.focus_scopes.keys().copied());
let ids: Vec<_> = ids.into_iter().collect();
```
Suggested fix: Drop the HashSet and Vec. refresh(id) only removes from dependencies and focus_scopes, so clearing both maps is equivalent: replace lines 390-396 with `self.dependencies.clear();` and `self.focus_scopes.clear();` (keep the comment about focus-only builders, rephrased to say the maps are cleared wholesale).

### A131-03 - crates/incular-runtime/src/environment.rs:502 (§2, low)
Evidence:
```
let owner = scope.element.map_or_else(
    || "the application root".to_owned(),
    |id| format!("element {id:?}"),
```
Suggested fix: Build the owner String only on the first report. Key REPORTED_BUILD_WRITES by the owner itself (`RefCell<HashSet<Option<ElementId>>>`), test `reported.borrow_mut().insert(scope.element)`, and format the message string only inside the `if first` branch. Per-write allocation then happens only when the write is the first for that owner, not on every build-time write.

### A131-04 - crates/incular-runtime/src/environment.rs:947 (§18, low)
Evidence:
```
#[cfg_attr(not(feature = "devtools"), allow(dead_code))]
#[derive(Clone)]
pub struct SignalRegistration {
```
Suggested fix: Replace `allow` with `expect` and a reason: `#[cfg_attr(not(feature = "devtools"), expect(dead_code, reason = "registrations are only read by the DevTools registry"))]`. Verify with `cargo check` both with and without the devtools feature, since expect fails if the lint does not fire under some configuration.

### A136-01 - crates/incular-runtime/src/frame/text_actions.rs:104 (§2, medium)
Evidence:
```
let value = controller.value();
let text = value.text.as_str();
```
Suggested fix: `controller.value()` clones the whole TextEditingValue (including the full text String) on every text key event. The clone exists only to avoid holding the controller's RefCell borrow while the intent mutates the controller. Compute the target offset/selection inside `controller.with_value(|value| ...)` (which returns only a usize or TextSelection) and call the mutating controller method after the closure returns. Only the Transpose branch, which needs the text bytes, should read `value.text` in its own `with_value` call.

### A136-03 - crates/incular-runtime/src/frame/text_actions.rs:384 (§2, medium)
Evidence:
```
let Some((text, selection, foreign_anchor)) =
    self.tree.selectable_text_editing_value(label)
```
Suggested fix: `selectable_text_editing_value` (crates/incular-widgets/src/tree/text.rs:944) returns an owned `String` cloned from the widget's text via `selectable_text_value` (text.rs:1152), so every arrow/shift key press on a SelectableText copies the whole text. Return a borrow or a shared handle instead (for example `Arc<str>` stored on the widget kind, or a `with_selectable_text_editing_value(label, |text, selection, foreign| ...)` callback), and compute `next` inside that scope. Needs the tree-side change in incular-widgets as well.

### A136-04 - crates/incular-runtime/src/frame/text_actions.rs:566 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
fn line_boundary_at(
```
Suggested fix: Replace with `#[expect(clippy::too_many_arguments, reason = "the line-boundary query needs the caret offset, affinity, direction, wrap flag and obscured state")]`.

### A136-02 - crates/incular-runtime/src/frame/text_actions.rs:589 (§2, medium)
Evidence:
```
if forward {
    (controller.text().len(), TextAffinity::Upstream)
```
Suggested fix: `controller.text()` allocates a copy of the whole field text only to read its length. Replace with the borrow-based accessor: `(controller.with_value(|value| value.text.len()), TextAffinity::Upstream)`.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
