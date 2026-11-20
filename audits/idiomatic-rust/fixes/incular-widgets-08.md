# Fix brief incular-widgets-08

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/app_shell/app.rs`. Other agents are editing other files at
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
### A198-04 - crates/incular-widgets/src/app_shell/app.rs:329 (§2, medium)
Evidence:
```
let information = retained.current_route();
```
Suggested fix: Build from the route already read: add `fn build_route(&self, information: &RouteInformation) -> Widget` (the `widget_for(..).unwrap_or_else(..)` body of build_current_widget), then in the closure do `let child = retained.build_route(&information); Widget::environment_scope(WidgetsAppData { current_route: information, .. }, child)`. build_current_widget becomes ensure_initialized + current_route + build_route. This removes the second RouteInformation clone and lookup per rebuild (line 336 calls build_current_widget, which re-reads the route).

### A198-02 - crates/incular-widgets/src/app_shell/app.rs:355 (§2, medium)
Evidence:
```
.get(&route_path(information.location()))
```
Suggested fix: Normalize once before the state borrow: `let path = route_path(information.location());` then use `.get(&path)` in the block and `if path == "/" && let Some(home) = home` below, so route_path (which allocates) runs once per lookup instead of twice (line 359 repeats it).

### A198-06 - crates/incular-widgets/src/app_shell/app.rs:374 (§11, medium)
Evidence:
```
controller.widget_for(information).is_some()
```
Suggested fix: Existence checks should not run user builders: test `home` (for "/") and `routes.contains_key(&path)` directly, and call `generate_route` only when neither matches, since it is the only source that can report existence without building. Discarding a built Widget per navigation (push_inner, replace_inner, initial_stack loop) builds and drops a subtree and runs user code with side effects.

### A198-11 - crates/incular-widgets/src/app_shell/app.rs:419 (§14, low)
Evidence:
```
state.revision.set(
```
Suggested fix: The revision bump `state.revision.set(state.revision.get().checked_add(1).expect("revision exhausted"))` is repeated at lines 401, 419, 480, 525, 560 and 593, plus invalidate(). Add a `bump_revision(&self)` method on WidgetsAppState and call it everywhere. The stack-restore branch in ensure_initialized (lines 393-410) also duplicates the default-initialization branch (lines 414-426), so merge them.

### A198-08 - crates/incular-widgets/src/app_shell/app.rs:573 (§2, low)
Evidence:
```
information.clone(),
```
Suggested fix: `information` is not used after this report in replace_stack (the notify call reads the stack), so pass it by value: `information,` instead of `information.clone(),`.

### A198-05 - crates/incular-widgets/src/app_shell/app.rs:617 (§2, medium)
Evidence:
```
(state.restoration.clone(), state.stack.clone())
```
Suggested fix: Return early when restoration is absent and serialize under the borrow: `let state = inner.borrow(); let Some(restoration) = state.restoration.clone() else { return }; let value = state.stack.iter().map(RouteInformation::to_json).collect::<Vec<_>>(); drop(state); restoration.scope.set_json(&restoration.key, value.into());`. The whole stack is currently cloned on every push, pop and replace even when restoration is not configured. Keep the drop before set_json, since it can re-enter the controller.

### A198-09 - crates/incular-widgets/src/app_shell/app.rs:920 (§2, low)
Evidence:
```
Localizations::new(locale.clone(), SharedCatalog(catalog.clone()), child).into();
```
Suggested fix: `locale` is already an owned Locale from the `&& let Some(locale) = locale.clone()` binding, so move it: `Localizations::new(locale, SharedCatalog(catalog.clone()), child).into()`.

### A198-10 - crates/incular-widgets/src/app_shell/app.rs:963 (§2, low)
Evidence:
```
environment: environment.clone().normalized(),
```
Suggested fix: `environment` is a local whose last use is this line, so write `environment,` instead of cloning it (it is already normalized on line 951).

### A198-03 - crates/incular-widgets/src/app_shell/app.rs:987 (§2, medium)
Evidence:
```
|| normalized.clone(),
|(index, _)| normalized[..index].to_owned(),
```
Suggested fix: `normalized` is owned and unused after the lookup, so return it instead of cloning: `match normalized.char_indices().find(|(_, c)| matches!(c, '?' | '#')) { Some((index, _)) => normalized[..index].to_owned(), None => normalized }`.

### A198-07 - crates/incular-widgets/src/app_shell/app.rs:997 (§2, low)
Evidence:
```
(1..=segments.len())
.map(|count| format!("/{}", segments[..count].join("/")))
```
Suggested fix: Build the prefixes incrementally into one buffer: `let mut prefix = String::new(); segments.iter().map(|segment| { prefix.push('/'); prefix.push_str(segment); prefix.clone() }).collect()`. This drops the per-prefix join and format! intermediates (two allocations per prefix) and keeps the same output.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
