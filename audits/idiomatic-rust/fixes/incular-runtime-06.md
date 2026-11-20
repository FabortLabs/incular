# Fix brief incular-runtime-06

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-runtime/src/devtools_inspection.rs`, `crates/incular-runtime/src/reactive.rs`, `crates/incular-runtime/src/application_activations.rs`, `crates/incular-runtime/src/request_registry.rs`, `crates/incular-runtime/src/request_channel.rs`, `crates/incular-runtime/src/route_focus.rs`, `crates/incular-runtime/src/tasks.rs`, `crates/incular-runtime/src/app_log.rs`. Other agents are editing other files at
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
### A130-02 - crates/incular-runtime/src/devtools_inspection.rs:15 (§26, low)
Evidence:
```
macro_rules! with_tree {
    ($self:ident, $window:ident, |$tree:ident| $read:expr) => {{
```
Suggested fix: Replace the macro with a private generic method on Application, e.g. `fn with_window_tree<T>(&self, window: DevWindowId, read: impl FnOnce(&<tree type returned by record.runtime.tree()>) -> T) -> Result<T, ErrorCode>` that performs the registry borrow, stale-ID lookup and calls `read(record.runtime.tree())`. The four callers become `self.with_window_tree(window, |tree| tree.devtools_runtime_counters())`.

### A130-01 - crates/incular-runtime/src/devtools_inspection.rs:18 (§21, medium)
Evidence:
```
$window.index().saturating_sub(1) as u32,
$window.generation() as u32,
```
Suggested fix: Replace the truncating casts with checked conversions that map out-of-range wire IDs to a stale ID, e.g. in a function returning Result<T, ErrorCode>: `let index = u32::try_from(window.index().saturating_sub(1)).map_err(|_| ErrorCode::StaleId)?;` and `let generation = u32::try_from(window.generation()).map_err(|_| ErrorCode::StaleId)?;`, then `WindowId::from_parts(index, generation)`. Also decide whether index 0 (saturating_sub(1) maps it to 0) should be StaleId instead of aliasing window index 0.

### A140-01 - crates/incular-runtime/src/reactive.rs:310 (§2, medium)
Evidence:
```
detach_sources_for(binding.root, &self.sources);
sources.borrow_mut().remove(&root);
let entries = sources.entry(root).or_default();
```
Suggested fix: Keep the per-root Vec and clear it instead of removing the map entry, so the next recompute reuses its capacity. Add a clear-only helper for recompute, e.g. fn clear_sources_for(root, sources: &RefCell<HashMap<ReactiveRootId, Vec<TrackedDependency>>>) { if let Some(entries) = sources.borrow_mut().get_mut(&root) { entries.clear(); } } and call it at line 310 instead of detach_sources_for. Keep remove() in dispose_root (line 391), where the entry should really go.

### A140-02 - crates/incular-runtime/src/reactive.rs:334 (§2, medium)
Evidence:
```
let dependencies = self
    .sources
    .borrow()
```
Suggested fix: Only build the `dependencies` Vec when there are other roots to refresh. Compute `others` first, then `if !others.is_empty() { let dependencies: Vec<Rc<dyn Dependency>> = ...; for other in others { ... } }`. Today every recompute with a binding allocates this Vec even in the common single-root case, where it is discarded (the loop over `others` does nothing).

### A128-01 - crates/incular-runtime/src/application_activations.rs:160 (§2, medium)
Evidence:
```
queue.pending.pop_front().map(|activation| {
                (
                    activation,
                    queue
                        .listeners
                        .iter()
                        .filter_map(Weak::upgrade)
                        .collect::<Vec<_>>(),
```
Suggested fix: Hoist the snapshot buffer into ActivationState (add `scratch: Vec<ApplicationActivationListener>`). In drain, inside the existing borrow, take it with `std::mem::take(&mut queue.scratch)`, `clear()` it, and `extend(queue.listeners.iter().filter_map(Weak::upgrade))` instead of collecting. After the delivery loop, `state.queue.borrow_mut().scratch = listeners;` to return the capacity. The active flag guarantees no nested drain uses the scratch while it is taken, and the snapshot still avoids holding the RefCell borrow across callbacks.

### A141-01 - crates/incular-runtime/src/request_registry.rs:37 (§10, medium)
Evidence:
```
assert!(
    !self.entries.contains_key(&id),
    "native request identity reused"
```
Suggested fix: Use the map entry API so the duplicate check and insert share one hash lookup. Add `use std::collections::hash_map::Entry as MapEntry;` and replace the assert and the `self.entries.insert(...)` call with:
match self.entries.entry(id) {
    MapEntry::Occupied(_) => panic!("native request identity reused"),
    MapEntry::Vacant(slot) => {
        slot.insert(Entry { value, phase: RequestPhase::Queued });
    }
}

### A141-03 - crates/incular-runtime/src/request_channel.rs:92 (§2, low)
Evidence:
```
pub(crate) fn new(
        receiver: oneshot::Receiver<T>,
        abandoned: impl FnOnce() + Send + Sync + 'static,
```
Suggested fix: Make the abandonment callback a type parameter instead of a boxed trait object: `pub(crate) struct RequestReceiver<T, F: FnOnce() + Send + Sync> { receiver: Option<oneshot::Receiver<T>>, abandoned: Option<F> }`, `new(receiver, abandoned: F)` storing `Some(abandoned)`, and the Drop impl as `impl<T, F: FnOnce() + Send + Sync> Drop for RequestReceiver<T, F>`. This removes one heap allocation per native request. Callers are crate-private, so no public API change.

### A142-06 - crates/incular-runtime/src/route_focus.rs:161 (§2, medium)
Evidence:
```
|| !runtime.tree().focusable_elements().contains(&id)
|| owner_of(runtime.tree(), id)
```
Suggested fix: focusable_elements() walks the whole tree and returns a freshly allocated Vec, and orphaned() calls it for the one element being tested. restore_if_vacant runs on every reconciliation for a route whose record is Some, so this costs a full walk plus allocation there. Add a per-element predicate next to focusable_elements_recursive in crates/incular-widgets/src/tree/focus.rs (checks the same rule for one element by walking its ancestors) and use it here and in eligible() (line 146). Also in restore_saved, compute orphaned(current) once instead of at lines 122 and 131. Equivalence must be checked against focusable_elements_recursive, so this touches incular-widgets too.

### A147-01 - crates/incular-runtime/src/tasks.rs:844 (§2, low)
Evidence:
```
let mut work = Vec::new();
let mut processed = 0;
while processed < DEFAULT_UI_MESSAGE_BUDGET {
```
Suggested fix: Size the batch from the known queue depth so the pushes in the drain loop do not regrow: `let mut work = Vec::with_capacity(self.bridge.queued.load(Ordering::Acquire).min(DEFAULT_UI_MESSAGE_BUDGET));`. Allocation remains only on the message path (take_turn runs from process_runtime_work_at on user events, not every frame). A caller-owned buffer passed as &mut Vec<UiWork> would remove the per-turn allocation entirely if this path is measured to matter.

### U088-01 - crates/incular-runtime/src/app_log.rs:197 (§6, low)
Evidence:
```
.map(|q| !q.is_empty())
.unwrap_or(false)
```
Suggested fix: Replace the map and unwrap_or pair with is_ok_and, which states the poisoned-lock fallback in one call:

        global()
            .queue
            .lock()
            .is_ok_and(|q| !q.is_empty())


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
