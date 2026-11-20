# Fix brief incular-desktop-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-desktop/src/devtools_runner.rs`. Other agents are editing other files at
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
### A024-07 - crates/incular-desktop/src/devtools_runner.rs:291 (§2, low)
Evidence:
```
property.overridden = self
.property_overrides
.contains_key(&(id, property.name.clone()));
```
Suggested fix: Each property name is cloned into a tuple key only to look it up. Key property_overrides by widget id first (HashMap<DevWidgetId, HashMap<String, PropertyOverride>>) and look up with `.get(&id).is_some_and(|m| m.contains_key(property.name.as_str()))`. The same clone at line 301 (`get(&(id, name.clone()))`) goes away with that change.

### A024-04 - crates/incular-desktop/src/devtools_runner.rs:441 (§11, medium)
Evidence:
```
self.refresh_layout_bounds(application);
self.refresh_auxiliary_overlays(application);
```
Suggested fix: drain() already runs both refreshes, and frame_update() (lib.rs:1440) runs them again for each presented window, so overlay geometry is recomputed twice per frame when a wake and a frame coincide. Remove these two calls from drain(); the next presented frame refreshes the overlays, and drain already requests a redraw when overlay options change (repaint_overlays).

### A024-08 - crates/incular-desktop/src/devtools_runner.rs:485 (other, low)
Evidence:
```
let overrides = std::mem::take(&mut self.property_overrides);
for ((id, name), property) in overrides {
let _ = application.devtools_revert_property(id, &name, &property.original);
```
Suggested fix: reset_session repeats the revert loop from ResetOverrides (lines 430-436). Extract `fn revert_overrides(&mut self, application: &mut incular_runtime::Application)` containing the take-and-revert loop and call it from both places.

### A024-05 - crates/incular-desktop/src/devtools_runner.rs:497 (§11, medium)
Evidence:
```
for window in application.active_window_ids() {
if self.options.contains(&DebugOption::HitTestRegions) {
```
Suggested fix: frame_update(application, window) calls refresh_auxiliary_overlays, which recomputes overlay geometry for every active window, so with W presented windows each frame costs W times the work. Add a window parameter (`fn refresh_auxiliary_overlays(&mut self, application, window)`) and refresh only that window's entries, clearing just that window's map entries instead of all four maps. Callers in drain() must loop over windows explicitly if kept.

### A024-03 - crates/incular-desktop/src/devtools_runner.rs:605 (§2, medium)
Evidence:
```
let mut next = HashMap::new();
let mut live = std::collections::HashSet::new();
```
Suggested fix: refresh_phase_flashes runs once per presented frame while flash/rainbow options are on and allocates a new HashMap and HashSet every time, then drops the old counters map. Keep both as scratch fields on DevToolsState (for example phase_scratch: HashMap<DevWidgetId, [u64; 5]> and live_scratch: HashSet<DevWidgetId>), call clear() at the start, and finish with std::mem::swap(previous, next) instead of `*previous = next;`. Split borrows of distinct fields already work in this function.

### A024-02 - crates/incular-desktop/src/devtools_runner.rs:896 (§2, high)
Evidence:
```
let alive: Vec<([f32; 4], f32)> = flashes
```
Suggested fix: Drop the per-phase Vec in paint_flashes (runs on every paint while flashes fade). Build the filtered iterator in a closure and call it twice: `let alive = || flashes.values().filter_map(|flash| Some((flash.bounds, fade(flash.started[index])?))).filter(|(bounds, _)| bounds[2] > 0. && bounds[3] > 0.);` then `let leaf_tests = alive().count() <= MAX_LEAF_TESTS;` and loop with `for (position, (bounds, fade)) in alive().enumerate()`, using `alive().enumerate().any(...)` for the leaf test. Same complexity as today, no heap allocation.

### A025-02 - crates/incular-desktop/src/devtools_runner.rs:896 (§2, medium)
Evidence:
```
let alive: Vec<([f32; 4], f32)> = flashes
    .values()
    .filter_map(|flash| Some((flash.bounds, fade(flash.started[index])?)))
```
Suggested fix: paint_flashes runs on every painted frame while a flash option is enabled and allocates a new Vec per enabled phase. Keep a scratch field `flash_scratch: Vec<([f32; 4], f32)>` on the runner, `clear()` it at the start of each phase and push into it (capacity is kept across frames). This needs `&mut self` on paint_flashes and paint_overlay; if the caller only holds `&self`, put the scratch behind a RefCell.

### A024-06 - crates/incular-desktop/src/devtools_runner.rs:944 (§2, medium)
Evidence:
```
let next = tree_nodes(&snapshot);
```
Suggested fix: tree_nodes clones every WidgetNode of a full snapshot (`(**root).clone()` and `rest.iter().cloned()`) on each 250 ms sample of subscribed trees, although snapshot is not used afterwards. Add an owned variant `fn tree_nodes_owned(deltas: Vec<TreeDelta>) -> HashMap<DevWidgetId, WidgetNode>` that moves the nodes out (`nodes.insert(root.id, *root)`, `nodes.extend(rest.into_iter().map(|node| (node.id, node)))`) and call it here; keep the borrowed version for remember_tree, which must return the deltas to the caller.

### A025-03 - crates/incular-desktop/src/devtools_runner.rs:1069 (§2, medium)
Evidence:
```
nodes.insert(root.id, (**root).clone());
nodes.extend(rest.iter().cloned().map(|node| (node.id, node)));
```
Suggested fix: Make tree_nodes consume its input: `fn tree_nodes(deltas: Vec<TreeDelta>) -> HashMap<DevWidgetId, WidgetNode>`, pre-sized with `HashMap::with_capacity`, and move `*root` and the `rest` nodes out of each Snapshot delta. The 4 Hz sampler at line 944 then passes `snapshot` by value with no per-node clone of a whole tree. The GetWidgetTree call site (line 278, outside this range) passes `deltas.clone()` because the response still needs `deltas`.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
