# Fix brief incular-widgets-03

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/reconciliation.rs`. Other agents are editing other files at
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
### A295-09 - crates/incular-widgets/src/tree/reconciliation.rs:73 (§2, medium)
Evidence:
```
if let Some(auxiliary) = element.auxiliary_state.as_mut() {
auxiliary.edit_transform_subscription = None;
auxiliary.edit_changed_subscription = None;
```
Suggested fix: sync_edit_callbacks is called from every non-skipped update of a TextField (line 750) and tears down both subscriptions and registers new boxed subscriptions even when the transform and changed callbacks are the same Rc values as before. Compare the old and new spec callbacks (Rc::ptr_eq on the transform and changed handles, and the controller) and keep the existing subscriptions when they match.

### A295-01 - crates/incular-widgets/src/tree/reconciliation.rs:840 (§2, high)
Evidence:
```
let previous = self
.element_live(id, "retained element must remain live")
.children
```
Suggested fix: Every non-skipped rebuild of an element with static children clones its child Vec (here) and collects `desired` into a new Vec (line 844). Avoid both per update: move the mounted list out with `std::mem::take(&mut element.children)` before reconciling and restore it on the error path, and let reconcile_children take `desired` as a borrowed iterator (or a tree-owned scratch buffer) instead of a fresh Vec<&Widget>.

### A296-01 - crates/incular-widgets/src/tree/reconciliation.rs:840 (§19, medium)
Evidence:
```
.children
.clone();
let desired = widget.children_refs().into_iter().collect::<Vec<_>>();
```
Suggested fix: Move the child list out instead of cloning it: `let previous = std::mem::take(&mut self.element_live_mut(id, <same message>).children);`, restore `previous` into `.children` on the error path of `reconcile_children`, then keep the existing `reconciled != previous` compare. Removes one Vec clone per rebuilt element with static children.

### A295-02 - crates/incular-widgets/src/tree/reconciliation.rs:879 (§2, medium)
Evidence:
```
let mut next = Vec::with_capacity(new_end);
while start < old_end && start < new_end && self.compatible(previous[start], desired[start])
{
```
Suggested fix: `next` is allocated on every call, but the caller (line 846) discards it when `reconciled == previous`, which is the steady state. Return an outcome that says whether the child list changed, and only build the `next` vector once a non-prefix difference is found (or write the prefix into a tree-owned scratch buffer).

### A296-02 - crates/incular-widgets/src/tree/reconciliation.rs:902 (§2, medium)
Evidence:
```
let mut keyed: HashMap<Key, (ElementId, usize)> = HashMap::new();
let mut unkeyed_ids: Vec<(ElementId, usize)> = Vec::new();
let mut used = std::collections::HashSet::new();
```
Suggested fix: On the no-keys path (`else` at line 923) do not fill `unkeyed_ids`; iterate `old_middle.iter().enumerate()` directly, mapping to `(id, position)`. Replace `used` (HashSet<ElementId>) and `kept` (HashSet<ElementId>) with `Vec<bool>` indexed by old middle position, since the candidate already carries its old position. Result: no per-call Vec or hash-set allocation in the common unkeyed rebuild.

### A295-08 - crates/incular-widgets/src/tree/reconciliation.rs:915 (§2, medium)
Evidence:
```
.and_then(|e| e.widget.key().cloned())
```
Suggested fix: Key::String is an owned String, so `.cloned()` allocates for every string-keyed old child on every middle reconciliation (line 915), and again for every keyed child on every dynamic reconciliation (line 1195). Index the keyed map by a borrowed or cheap identity (e.g. intern keys as Rc<str>/u64, or store the position and compare `element.widget.key()` through a reference in a second pass) so no String is copied per frame.

### A295-07 - crates/incular-widgets/src/tree/reconciliation.rs:929 (§2, medium)
Evidence:
```
let mut used = std::collections::HashSet::new();
used.insert(id);
let mut kept = std::collections::HashSet::new();
```
Suggested fix: Only runs when the middle section is non-empty, but then it allocates a SipHash HashSet and inserts every reused child. `used` can be a `Vec<bool>` of length old_middle.len() indexed by the old position already stored in `keyed`/`unkeyed_ids` (line 944 would set `used[old_position] = true`), and `kept` can be the same kind of vector. This also removes the inconsistent std hasher when the file already uses FastHashSet (line 1057).

### A296-04 - crates/incular-widgets/src/tree/reconciliation.rs:1000 (§19, medium)
Evidence:
```
let old_ids = {
element.sliver_child_ids().to_vec()
};
```
Suggested fix: Take the retained ids instead of copying them: `std::mem::take(&mut scrolling.sliver_child_ids)` before `reconcile_dynamic_children`, restoring it on the error path. The success path already overwrites `sliver_child_ids` (line 1073). Same pattern applies to `advanced_child_keys().to_vec()` in `reconcile_advanced_children` (line 1095). Both run on every sliver or advanced relayout.

### A295-03 - crates/incular-widgets/src/tree/reconciliation.rs:1002 (§2, high)
Evidence:
```
element.sliver_child_ids().to_vec()
let desired = layout
.children
```
Suggested fix: reconcile_sliver_children runs per sliver layout pass and allocates twice before reconciling: the old key Vec via `.to_vec()` (only to dodge the &mut self borrow) and a `desired` Vec with one cloned widget per child (lines 1004-1011). Store the key list as `Arc<[K]>`/`Rc<[K]>` in the scrolling state so the borrow is a refcount bump, and reuse a tree-owned scratch Vec for `desired` (clear + extend) instead of collecting a new one each pass.

### A295-06 - crates/incular-widgets/src/tree/reconciliation.rs:1056 (§2, high)
Evidence:
```
let mut next_semantic_indices = Vec::with_capacity(layout.children.len());
let mut overlays = FastHashSet::default();
```
Suggested fix: Both buffers are created per sliver pass and then stored into scrolling state (lines 1074-1075), dropping the previous pass's buffers. Take the stored Vec and set out with `std::mem::take`, `clear()` them, refill, and store them back so steady-state passes allocate nothing.

### A296-05 - crates/incular-widgets/src/tree/reconciliation.rs:1056 (§2, medium)
Evidence:
```
let mut next_semantic_indices = Vec::with_capacity(layout.children.len());
let mut overlays = FastHashSet::default();
for child in &layout.children {
```
Suggested fix: Reuse the buffers already stored in the element's scrolling state: inside the `element_live_mut` block, `let scrolling = element.scrolling_state_mut(); scrolling.sliver_child_semantic_indices.clear(); scrolling.sliver_child_semantic_indices.extend(layout.children.iter().map(...));` and `scrolling.sliver_overlay_ids.clear();` followed by the same inserts. The current code allocates a fresh Vec and set per sliver layout and drops the old ones.

### A295-04 - crates/incular-widgets/src/tree/reconciliation.rs:1095 (§2, high)
Evidence:
```
element.advanced_child_keys().to_vec()
};
let desired = desired
```
Suggested fix: Same pattern as the sliver path: `old_keys` is copied with `.to_vec()` (line 1095) and `desired` is re-collected into a new Vec of DesiredDynamicChild (lines 1097-1100) on every advanced-scrolling layout pass. Share the key list (Arc<[K]>) or take it out with `std::mem::take` and restore it, and accept the converted Vec from a reusable scratch buffer rather than allocating each pass.

### A295-05 - crates/incular-widgets/src/tree/reconciliation.rs:1145 (§2, high)
Evidence:
```
let mut seen = HashSet::with_capacity(desired.len());
.collect::<HashMap<_, _>>();
let old_positions: HashMap<ElementId, usize> = old_children
```
Suggested fix: reconcile_dynamic_children_inner builds five hash containers and one Vec clone on every call, i.e. on every sliver or advanced-scroll materialization: `seen` (1145), `old_children` clone (1173-1176), `existing` (1178-1182, which also clones every K), `keyed_index` with one Vec per key (1190-1199), `old_positions` (1207-1212), and `retained` (1213). Fold the duplicate-key check, the old-position map and the retained set into one pass over reusable scratch containers owned by the tree (cleared each call, capacity kept), and index old children by position instead of hashing them.

### A296-06 - crates/incular-widgets/src/tree/reconciliation.rs:1173 (§2, medium)
Evidence:
```
let old_children = self
    .element_live(owner, "dynamic child owner must remain live")
    .children
    .clone();
```
Suggested fix: Keep the clone only where a mutable borrow forces it, and build the lookups from it once. The `existing` map (lines 1178-1182) clones every old key with `.cloned()`; build it as `HashMap<&K, ElementId>` over `old_keys` (borrowed from the caller, not from self) and look up with `existing.get(&key)`. The `old_positions` map (lines 1207-1212) can be a `Vec<usize>` lookup keyed by position if the `keyed_index` pass is merged into the same loop over `old_children`.

### A296-07 - crates/incular-widgets/src/tree/reconciliation.rs:1195 (§2, high)
Evidence:
```
.and_then(|element| element.widget.key().cloned())
{
keyed_index.entry(key).or_default().push(id);
```
Suggested fix: Per keyed element this clones the key (String keys allocate) and allocates a fresh Vec bucket through `or_default()`. Use the by-reference index described for reconcile_children (`HashMap<&K..>` or `HashMap<&Key, ElementId>` over `desired`) so no key is cloned, and keep old-sibling buckets only if duplicate old keys are possible (check that case before dropping the Vec). Reuse the scratch map across calls by clearing it.

### A296-08 - crates/incular-widgets/src/tree/reconciliation.rs:1454 (§2, low)
Evidence:
```
let reconciled = self.reconcile_dynamic_children(
element_id,
&old_keys,
vec![DesiredDynamicChild {
```
Suggested fix: `reconcile_dynamic_children` takes a Vec only so the single-child case allocates one Vec per layout-builder rebuild. Accept a `impl IntoIterator<Item = DesiredDynamicChild<K>>` (or a single-child fast path) so the one-element list is not heap-allocated.

### A297-03 - crates/incular-widgets/src/tree/reconciliation.rs:1953 (§2, low)
Evidence:
```
let mut work = vec![(id, viewport_depth)];
while let Some((current, depth)) = work.pop() {
    let Some(element) = self.elements.get(current.0) else {
```
Suggested fix: Take the traversal stack from the caller as a `work: &mut Vec<(ElementId, usize)>` parameter, `clear()` it, and push the root. The only caller (reconciliation.rs around line 1663-1664) creates `let mut sources = Vec::new();` per listener, so hoist one `sources` and one `work` buffer above that loop and pass both in. Only the notification subscription refresh calls this, so the cost is per topology change, not per frame.

### A297-01 - crates/incular-widgets/src/tree/reconciliation.rs:2309 (§2, high)
Evidence:
```
let render_children: Vec<_> = element
    .children
    .iter()
```
Suggested fix: Keep a tree-owned scratch buffer (e.g. a `Vec<RenderObjectId>` field on the tree). At the top of sync_render_children, `let mut render_children = std::mem::take(&mut self.render_children_scratch); render_children.clear(); render_children.extend(element.children.iter().filter_map(|child| self.render_id(*child)));`, and restore it with `self.render_children_scratch = render_children;` before returning. Keep `node.children.clone_from(&render_children)` as is. Alternatively, compare the filtered iterator with `node.children` first and only materialize a Vec when it differs. Behavior is unchanged.

### A297-02 - crates/incular-widgets/src/tree/reconciliation.rs:2355 (§2, medium)
Evidence:
```
RenderKind::IndexedStack { index, .. } => {
    child_layers = child_layers.get(index).copied().into_iter().collect();
}
```
Suggested fix: Reuse the existing buffer instead of allocating a second Vec: `let selected = child_layers.get(index).copied(); child_layers.clear(); child_layers.extend(selected);`. The same applies to the SliverViewport reorder at lines 2357-2363, which can clear and refill `child_layers` from a snapshot instead of collecting into a new Vec.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
