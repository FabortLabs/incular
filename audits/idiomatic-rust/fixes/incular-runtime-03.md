# Fix brief incular-runtime-03

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-runtime/src/route_outlet.rs`, `crates/incular-runtime/src/application.rs`. Other agents are editing other files at
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
### A143-05 - crates/incular-runtime/src/route_outlet.rs:456 (§2, medium)
Evidence:
```
let mut chain = Vec::new();
let mut cursor = Some(id);
while let Some(current) = cursor {
```
Suggested fix: The focus ownership oracle allocates an ancestor Vec on every call and then scans it with `chain.contains` for each nested and tagged outlet. The oracle runs on focus transitions and retries, not per element, so the cost is modest. Use a stack-allocated buffer (`smallvec::SmallVec<[ElementId; 16]>` if incular-runtime already depends on smallvec) or a reusable buffer held in the outlet, or walk the ancestor chain once and compare each candidate root against the walk without storing it.

### A143-03 - crates/incular-runtime/src/route_outlet.rs:794 (§2, medium)
Evidence:
```
let unsupported: Vec<RouteId> = self
    .navigator
    .routes()
```
Suggested fix: Only reached when `host_overlay_routes` is false, but then it runs every frame through preflight_tree and clones the whole stack (`routes()` returns an owned Vec<Route>) just to find overlay entries. Check without cloning: iterate borrowed state through the non-cloning navigator accessor from the capture finding, returning Err only when a matching route exists, and collect the ids into the error only on that failure path.

### A144-12 - crates/incular-runtime/src/route_outlet.rs:951 (§2, low)
Evidence:
```
let mut children = Vec::new();
// A replaced only route stays under its replacement.
for leaving in exiting
```
Suggested fix: Length is known up front: let mut children = Vec::with_capacity(routes.len() + exiting.len() * 2); (each route adds its veil, content and leaving children). Single growth step per frame instead of several reallocations.

### A144-11 - crates/incular-runtime/src/route_outlet.rs:1536 (§2, medium)
Evidence:
```
let stale = match &self.consumed {
    None => !self.navigator.routes().is_empty(),
```
Suggested fix: Builds a full Vec<Route> snapshot only to test emptiness, on every frame where nothing has been consumed yet. Add a non-allocating emptiness check on the navigator (for example Navigator::is_empty, reading state.routes.is_empty()) in incular-navigation and call it here.

### A144-10 - crates/incular-runtime/src/route_outlet.rs:1585 (§2, medium)
Evidence:
```
for nested in self.live_nested() {
    nested.borrow_mut().begin_frame(runtime, now);
}
```
Suggested fix: live_nested (line 653) collects a Vec<Rc<RefCell<Self>>> on each call. It runs per frame in begin_frame (1585), reconcile_tree (1595), reset_signaled_tree (1553) and flag_stale_frames (1543), and allocates whenever a nested outlet exists. Iterate the Weak list directly, for example for weak in self.nested.borrow().iter() { if let Some(nested) = weak.upgrade() { ... } }, keeping the nested borrow short, or keep a reusable scratch Vec. Verify that no callee mutates self.nested while the borrow is held.

### A127-05 - crates/incular-runtime/src/application.rs:2021 (§2, low)
Evidence:
```
let closed_restoration = record.restoration.clone();
```
Suggested fix: `let closed_restoration = record.restoration.take();` - `record` is an owned `mut` binding and its `restoration` field is not read again in `close_window`, so the metadata (which owns strings) can be moved out instead of cloned.

### A127-01 - crates/incular-runtime/src/application.rs:2042 (§2, medium)
Evidence:
```
match command.operation.clone() {
```
Suggested fix: Drop the clone and match by reference: change to `match &command.operation {` (the `operation =>` arm becomes a `&WindowOperation`). Before the native-commands push at line 2147, capture `let hides = matches!(operation, WindowOperation::SetVisible(false));` and use `hides` at line 2150 instead of `matches!(operation, ...)`, so the borrow of `command` ends before `command` is moved into `NativeWindowCommand::Operate`.

### A127-03 - crates/incular-runtime/src/application.rs:2198 (§19/§3, low)
Evidence:
```
let request_ids = self
.pending_native_requests
.iter()
```
Suggested fix: Keep the collect only if the pending-request map has no `retain`/`extract_if`-style removal; if it exposes one, remove and fail matching requests in a single pass. The same borrow-dodging collect appears at lines 2363-2369 in `fail_file_dialog_requests_for_window` and should be fixed in the same way. Window-close path only, so this is low severity.

### A127-02 - crates/incular-runtime/src/application.rs:2470 (§19/§2, low)
Evidence:
```
let abandoned: Vec<_> = self
.pending_global_shortcut_requests
.iter()
```
Suggested fix: Add an early return before the collect so the steady-state path never builds a Vec: `if !self.pending_global_shortcut_requests.iter().any(|(_, pending)| pending.reply.is_abandoned()) { return; }` placed right after `drain_queued(...)`. The collect (needed to avoid mutating the map while iterating) then runs only when an abandoned request exists.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
