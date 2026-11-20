# Fix brief incular-navigation-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-navigation/src/navigator.rs`, `crates/incular-navigation/src/registry.rs`. Other agents are editing other files at
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
### A108-01 - crates/incular-navigation/src/navigator.rs:330 (§2, low)
Evidence:
```
let lifetimes: Vec<RouteLifetime> = self
    .routes
    .iter()
```
Suggested fix: for entry in &self.routes {
    entry.lifetime.end();
}

### A109-04 - crates/incular-navigation/src/navigator.rs:1295 (§3, low)
Evidence:
```
let children: Vec<Rc<RefCell<BackDispatcherState>>> = node
    .borrow()
    .children
```
Suggested fix: Collect nothing: the Vec is only fed straight into `stack.extend`. Replace the `let children ... collect();` and `stack.extend(children);` pair with `stack.extend(node.borrow().children.iter().filter_map(std::rc::Weak::upgrade));`. The `Ref` temporary is dropped at the end of that statement, and `stack` is a separate local, so the borrow is safe.

### A110-01 - crates/incular-navigation/src/registry.rs:86 (§10, low)
Evidence:
```
if builders.contains_key(&route_id) {
    return Err(RestorableRouteRegistrationError::DuplicateRouteId(route_id));
}
```
Suggested fix: Replace the contains_key check and the later insert with one entry lookup (route_id.clone() is still needed because route_id is returned):

let builder: RestorableRouteBuilder = Rc::new(move |arguments: &Value| {
    incular_core::hot_reload::call_with(&mut &builder, arguments)
});
match builders.entry(route_id.clone()) {
    Entry::Occupied(_) => Err(RestorableRouteRegistrationError::DuplicateRouteId(route_id)),
    Entry::Vacant(slot) => {
        slot.insert(RestorableRouteDefinition { builder, clear_scope_on_pop });
        Ok(route_id)
    }
}

with `use std::collections::hash_map::Entry;` added (HashMap already imported). Remove the old contains_key block and the old insert call.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
