# Fix brief incular-widgets-06

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/semantics.rs`. Other agents are editing other files at
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
### A301-06 - crates/incular-widgets/src/tree/semantics.rs:40 (§18, low)
Evidence:
```
// Boxing the full build would add an allocation per rebuilt node.
#[allow(clippy::large_enum_variant)]
pub(super) enum Collected {
```
Suggested fix: Turn the allow into an expect that carries its reason, and drop the separate comment:
#[expect(clippy::large_enum_variant, reason = "Boxing the full build would add an allocation per rebuilt node.")]
Run cargo clippy afterwards to confirm the lint still fires, since expect fails when it does not.

### A301-07 - crates/incular-widgets/src/tree/semantics.rs:60 (§6, low)
Evidence:
```
const TOP_LEVEL: u32 = u32::MAX;
...
const RETAINED_LIST: u32 = u32::MAX - 1;
```
Suggested fix: Parent is encoded as a u32 with two sentinel values, both of which are read back by comparison (for example edges[cursor].0 == index as u32 at line 459 and parent != RETAINED_LIST at line 216). Replace them with an enum carried by SemanticVisit::parent and the edges tuple:
#[derive(Clone, Copy, PartialEq, Eq)]
enum SemanticParent { TopLevel, RetainedList, Build(u32) }
Note that the edge tuple grows from 4 to 8 bytes, so measure the effect on the semantics pass before adopting it.

### A301-01 - crates/incular-widgets/src/tree/semantics.rs:126 (§2, high)
Evidence:
```
let scrolls: Vec<ElementId> = self
.tracked
.scroll_views
```
Suggested fix: Allocates a Vec on every frame whose scroll offset changed. Replace with an index loop, which also avoids the borrow conflict that forced the collect (mark_render_semantics takes &mut self):
for index in 0..self.tracked.scroll_views.len() {
    let id = self.tracked.scroll_views[index];
    if !self.semantic_ids.contains_key(&id) { continue; }
    if let Some(render) = self.elements.get(id.0).map(|entry| entry.render) {
        self.mark_render_semantics(render, SemanticsDirty::Node);
    }
}
(If scroll_views is not a Vec, take a reusable buffer out of SemanticsScratch with std::mem::take instead, as the followers block below already does.)

### A301-05 - crates/incular-widgets/src/tree/semantics.rs:330 (§2, medium)
Evidence:
```
let anchors = |viewport: &Element| -> Vec<SliverChildId> {
viewport
.sliver_child_ids()
```
Suggested fix: The anchors closure collects into a Vec, and sliver_semantics_children calls it per visited sliver child (line 371, nested) and per viewport (line 339), allocating while a grouping applies. Return a lazy iterator instead of a Vec, and rebuild it where it is needed:
let anchors = |viewport: &Element| viewport.sliver_child_ids().iter().copied().filter(|id| crate::scrolling::is_semantics_anchor(*id));
Then replace anchors.is_empty() at line 340 with anchors(entry).next().is_none(), and make nested a closure call at line 384 instead of a stored Vec.

### A301-04 - crates/incular-widgets/src/tree/semantics.rs:365 (§3, medium)
Evidence:
```
let slot = parent.children.iter().position(|child| *child == element)?;
let anchor = *ids.get(slot)?;
if !crate::scrolling::is_semantics_anchor(anchor) {
```
Suggested fix: This linear scan runs for every child of every SliverViewport visited in a pass, so a viewport with N visited children costs O(N^2). The same scan is repeated at line 658 in collect_semantics_inner. Have the parent supply the slot: add a slot: u32 field to SemanticVisit, filled while the parent enumerates its children, and use it here and at line 658 instead of position(). Keep position() only as a fallback where no slot is available.

### A301-02 - crates/incular-widgets/src/tree/semantics.rs:424 (§2, medium)
Evidence:
```
label: build.label.clone(),
value: build.value.clone(),
description: build.description.clone(),
```
Suggested fix: Newly inserted nodes clone every label, value, description, state and action list here, and then the drain loop in the same function moves the same values again through semantics.update. The comment at lines 453-454 claims one copy only, but this loop makes a second one. Insert a placeholder with the empty value of each field, since the drain loop writes all content right after:
let id = self.semantics.insert(SemanticNode {
    id: SemanticNodeId(ArenaId::from_parts(0, 0)),
    role: build.role,
    label: Default::default(),
    value: Default::default(),
    description: Default::default(),
    bounds: build.bounds,
    state: Default::default(),
    actions: Default::default(),
    children: Vec::new(),
});
(Use the empty value of each field type if one of them has no Default impl. Check that the devtools revision comparison below is unaffected: it is taken after this insert, so it is.)

### A301-03 - crates/incular-widgets/src/tree/semantics.rs:490 (§2, medium)
Evidence:
```
children: child_ids.clone(),
}
);
```
Suggested fix: Every fully rebuilt node allocates a fresh children Vec from the reusable child_ids scratch. The Bounds arm at line 495 already passes child_ids as a slice to update_geometry. Give the full-update path the same slice-based write (in the semantics graph module) so the existing node's children Vec is overwritten in place and keeps its capacity, then drop the clone here. Verify first that update_geometry copies into the existing Vec and does not replace it.

### A302-06 - crates/incular-widgets/src/tree/semantics.rs:742 (§11, low)
Evidence:
```
focused: self.text_field_state_live(render_id).focused,
                        editable: spec.enabled && !spec.read_only,
```
Suggested fix: Reuse the `focused` value already computed at line 723 (`focused: focused,` or shorthand `focused,`) instead of looking up the same live text-field state a second time in the same arm.

### A302-05 - crates/incular-widgets/src/tree/semantics.rs:815 (§2, medium)
Evidence:
```
let explicit_description = entry
            .widget
            .semantic_properties()
```
Suggested fix: Remove the eager `explicit_description` binding. It clones the explicit description for every visited element, including role-less ones that never emit a SemanticBuild. Inside the emitting branch (line 929) use `.or_else(|| entry.widget.semantic_properties().explicit.as_ref().and_then(|semantics| semantics.description.clone()))` so the clone happens only when a node is emitted and the widget's own description is absent.

### A302-01 - crates/incular-widgets/src/tree/semantics.rs:992 (§2, medium)
Evidence:
```
let keys: Vec<Option<incular_semantics::OrdinalSortKey>> = group
            .iter()
            .map(|(_, _, element)| {
```
Suggested fix: Drop the per-group `keys` Vec and the `sort_key.clone()` in widget_sort_key. Change widget_sort_key to return Option<&incular_semantics::OrdinalSortKey> (`.and_then(|explicit| explicit.state.sort_key.as_ref())`), change sort_keys_compatible to take Option<&OrdinalSortKey>, and look keys up from self.elements inside the run loop and sort closure. Also test `group.iter().all(...)` for no keys before doing any work, so the common unkeyed group allocates nothing. Removes a String clone per keyed element and the Vec per group.

### A302-02 - crates/incular-widgets/src/tree/semantics.rs:1003 (§2, medium)
Evidence:
```
let mut order: Vec<usize> = (0..group.len()).collect();
        let mut run_start = 0;
```
Suggested fix: Sort each keyed run in place instead of building an index `order` Vec and a `sorted` Vec copied back (lines 1021-1022). Runs are contiguous and all their elements share one sort-key name (or are all unkeyed), so an in-place stable sort of group[run_start..run_end] with the key looked up from self.elements gives the same order and run boundaries. Delete `order`, `sorted` and copy_from_slice. Removes two allocations per sibling group that has more than one child.

### A302-03 - crates/incular-widgets/src/tree/semantics.rs:1056 (§2, medium)
Evidence:
```
let Some((viewport, controller, axis, reverse, _)) =
            self.scrollable_viewport_chain(element).into_iter().next()
```
Suggested fix: scrollable_viewport_chain (interaction.rs:432) builds and returns the whole chain Vec with cloned controllers and physics, and only the first entry is used. Add a `first_scrollable_viewport(&self, element) -> Option<(ElementId, ScrollController, Axis, bool, ScrollPhysics)>` in interaction.rs that runs the same walk and returns on the first push, and call it here. Behaviour is unchanged because only the first element of the chain is read.

### A302-04 - crates/incular-widgets/src/tree/semantics.rs:1107 (§2, medium)
Evidence:
```
let mut parts: Vec<String> = own.into_iter().filter(|label| !label.is_empty()).collect();
        if let Some(entry) = self.elements.get(element.0) {
            for &child in &entry.children {
```
Suggested fix: Collect the merged label into one String instead of a Vec<String> that is then joined. Thread `out: &mut String` (plus a flag for whether a separator is needed) through push_merged_labels and push each label with a separator. Make element_own_label return Option<Cow<'_, str>> (borrowing the label or text from the widget, Cow::Owned only for the Button widget_text case) so descendant labels are not cloned. Removes a per-label String clone, the Vec and the final join for every MergeSemantics node.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
