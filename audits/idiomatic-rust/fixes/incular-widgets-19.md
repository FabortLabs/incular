# Fix brief incular-widgets-19

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/retained.rs`, `crates/incular-widgets/src/tree/widget/constructors.rs`, `crates/incular-widgets/src/advanced_scrolling/draggable.rs`. Other agents are editing other files at
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
### A299-02 - crates/incular-widgets/src/tree/retained.rs:230 (§2, high)
Evidence:
```
let entries = self.transient_portal_entries();
let active = entries.iter().map(|entry| entry.0).collect::<HashSet<_>>();
self.layout_overlay_popups = entries
```
Suggested fix: resolve_transient_placements runs on every layout (layout.rs:113). When transients are open it allocates a Vec (transient_portal_entries), a fresh HashSet (`active`) and a fresh FastHashSet (`layout_overlay_popups`) each pass. Keep reusable scratch buffers on WidgetTree (e.g. `transient_scratch: Vec<_>`, `active_scratch: HashSet<_>`, and refill `layout_overlay_popups` with `clear()` + `extend(...)`) and clear/refill them instead of re-collecting.

### A299-05 - crates/incular-widgets/src/tree/retained.rs:416 (§2, medium)
Evidence:
```
let snapshots = self
.transient_surface_entries()
.into_iter()
```
Suggested fix: dismiss_transients_for_pointer runs on every pointer Down (frame.rs:1363) and, while a popup is open, allocates a Vec via transient_surface_entries, a HashMap of snapshots (416-420), a HashMap precise_content (423-437), a HashSet protected and a Vec frontier. Avoid the maps by iterating the entries Vec and doing linear lookups (open transients are few), or keep scratch buffers on WidgetTree. The same snapshots HashMap pattern is rebuilt per focus change in dismiss_transients_for_focus (lines 480-484).

### A299-04 - crates/incular-widgets/src/tree/retained.rs:555 (§6, medium)
Evidence:
```
let placement = self.transient_placements.get(&id).copied().unwrap_or(
RetainedTransientPlacement {
desired_size: content_rect.size,
```
Suggested fix: Use the lazy form so place_transient and root_layout_size() run only when no placement was recorded: `.copied().unwrap_or_else(|| RetainedTransientPlacement { desired_size: content_rect.size, result: place_transient(TransientPlacementInput { ... }) })`. Same result; the fallback is pure.

### A300-02 - crates/incular-widgets/src/tree/retained.rs:1013 (§11, high)
Evidence:
```
let bounds = self.element_bounds(id)?;
let mut visible = Some(bounds);
let mut ancestor = element.parent;
```
Suggested fix: Per view, per frame, the ancestor loop (lines 1016-1024) calls element_bounds(parent) for every ancestor, and each call goes through render_world_transform (crates/incular-widgets/src/tree/layout.rs:582), which builds a fresh `path` Vec (layout.rs:583) and walks to the root. Cost is O(depth) allocations and O(depth^2) transform work per platform view each frame. Fix: walk the ancestor chain once, root-first, accumulating the world transform with the same recurrence as render_world_transform (translation(node.offset), then child_content_transform for each non-leaf), and intersect each ancestor's transformed Rect::from_origin_size(Offset::ZERO, size). Use a reusable scratch buffer for the chain (or recurse) so no allocation happens per frame. Also make render_world_transform in layout.rs:583 avoid its per-call `Vec::new()`, since element_bounds and element_geometry use it everywhere.

### A307-03 - crates/incular-widgets/src/tree/widget/constructors.rs:443 (§2, low)
Evidence:
```
} else {
    Some(surface.label.clone())
}
```
Suggested fix: Move the label instead of cloning it when content is present: let (label, semantic_label) = match surface.content { Some(content) => { let semantic_label = if surface.label.is_empty() { content.semantic_text() } else { Some(surface.label) }; (content, semantic_label) } None => { let semantic_label = (!surface.label.is_empty()).then(|| surface.label.clone()); (Widget::padding(surface.padding, Widget::text_styled(surface.label, surface.label_style, TextAlign::Start)), semantic_label) } }. The clone is still needed on the no-content branch because the label is both semantic text and visible text there. Output is identical.

### A307-04 - crates/incular-widgets/src/tree/widget/constructors.rs:531 (§18, low)
Evidence:
```
#[must_use]
#[allow(clippy::too_many_arguments)]
pub(crate) fn text_configured(
```
Suggested fix: Replace #[allow(clippy::too_many_arguments)] with #[expect(clippy::too_many_arguments, reason = "text descriptor lowering passes every paragraph field explicitly")]. The same change applies to the #[allow] on image() at line 665. Both functions exceed the 7-argument threshold, so the expectation is fulfilled.

### A308-01 - crates/incular-widgets/src/tree/widget/constructors.rs:1073 (§18, low)
Evidence:
```
#[must_use]
#[allow(clippy::too_many_arguments)]
pub(crate) fn scroll_view_with_options(
```
Suggested fix: Replace the attribute with `#[expect(clippy::too_many_arguments, reason = "mirrors the Flutter ScrollView constructor parameters; a config struct would split the lowering path")]`.

### A308-02 - crates/incular-widgets/src/tree/widget/constructors.rs:1242 (§18, low)
Evidence:
```
#[allow(clippy::too_many_arguments)]
pub(crate) fn sliver_viewport_with_delegate_options(
```
Suggested fix: Replace the attribute with `#[expect(clippy::too_many_arguments, reason = "ten Flutter viewport parameters are lowered together; grouping them would only move the list")]`.

### A188-06 - crates/incular-widgets/src/advanced_scrolling/draggable.rs:713 (§2, medium)
Evidence:
```
mounted.snap = self.snap.clone();
drop(mounted);
self.controller.attach(&state.state);
```
Suggested fix: Skip the clone when unchanged: `if mounted.snap != self.snap { mounted.snap = self.snap.clone(); }` (DraggableSnap derives PartialEq). Alternatively store `Rc<DraggableSnap>` in both the config and SheetState so the clone is a refcount bump. `update_mounted_state` runs on each declarative update, so the Vec<f32> clone should not happen when the snap config is unchanged.

### A189-03 - crates/incular-widgets/src/advanced_scrolling/draggable.rs:713 (§2, high)
Evidence:
```
mounted.extent.has_changed = has_changed;
mounted.snap = self.snap.clone();
drop(mounted);
```
Suggested fix: Only replace the stored snap when it differs, so the steady state allocates nothing: `if mounted.snap != self.snap { mounted.snap = self.snap.clone(); }`. DraggableSnap derives PartialEq (line 490) and clones a Vec<f32>. update_mounted_state runs on every sheet layout pass (tree/layout.rs:255), so the unconditional clone allocates per layout.

### A188-04 - crates/incular-widgets/src/advanced_scrolling/draggable.rs:931 (§2, high)
Evidence:
```
(result, state.parent_controllers.clone())
};
inner_consumed = inner_result.consumed;
```
Suggested fix: Store `parent_controllers` as `Rc<[ScrollController]>` in SheetState (rebuilt in `set_parent_controllers`, line 839, a setup call) and clone the `Rc`, or have `incular_scroll::NestedScrollCoordinator` borrow `&[ScrollController]` instead of taking `Vec<ScrollController>` (line 935). Either removes the per-drag-sample Vec allocation on the overflow-into-parents path during user drags.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
