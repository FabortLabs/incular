# Fix brief incular-widgets-24

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/navigation/barrier.rs`, `crates/incular-widgets/src/overscroll_indicator.rs`, `crates/incular-widgets/src/scrolling/sliver_fade.rs`, `crates/incular-widgets/src/spell_check.rs`, `crates/incular-widgets/src/tree/focus.rs`, `crates/incular-widgets/src/tree/layout/behavior/sliver.rs`. Other agents are editing other files at
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
### A238-03 - crates/incular-widgets/src/navigation/barrier.rs:243 (§23, high)
Evidence:
```
let dismiss = {
let on_dismiss = on_dismiss.clone();
let dismissal_handler = dismissal_handler.clone();
```
Suggested fix: The builder passed to animated_widget_for runs again on each animation value change, and this block allocates a new Rc plus two Option<Rc> clones every time, although none of it depends on the animation value. Hoist the block above the `crate::explicit_animations::animated_widget_for(animation, move |_| {` call (`let dismiss: Rc<dyn Fn()> = Rc::new(move || { if let Some(callback) = &on_dismiss { callback(); } else if let Some(callback) = &dismissal_handler { let _ = callback(); } });`), so the builder only captures `dismiss` and uses `dismiss.clone()` (already done at the two existing use sites).

### A238-04 - crates/incular-widgets/src/navigation/barrier.rs:293 (§23, high)
Evidence:
```
let mut explicit = ExplicitSemantics::new(role);
if let Some(label) = &semantics_label {
explicit = explicit.label(label.clone());
```
Suggested fix: ExplicitSemantics (role, label, tap hint, Activate action) is rebuilt inside the per-animation-value builder, cloning semantics_label and semantics_on_tap_hint Strings on every build. None of these inputs depends on the animation value. Build the explicit semantics once before `animated_widget_for` (role, `.label(label.clone())`, `.description(hint.clone())`, `.actions([SemanticActionKind::Activate])` as applicable), then move it into the closure and reuse it there (use `.clone()` on it only if ExplicitSemantics is Clone; check incular-semantics before relying on that). This removes the per-build String allocations.

### A241-01 - crates/incular-widgets/src/overscroll_indicator.rs:593 (§11, high)
Evidence:
```
canvas.fill_path(
    Arc::new(circle(center, radius)),
    Color::rgba(color.red, color.green, color.blue, alpha),
```
Suggested fix: Build the unit circle once and reuse it. Store `Arc<incular_rendering::Path>` for the unit circle on GlowIndicatorState (or pass it into GlowController::paint) and place it with a transform: replace `circle(center, radius)` with the cached `unit_circle: Arc<Path>` and compose `CoreTransform::translation(center).then(CoreTransform::scale_non_uniform(radius, radius))` into the existing `transform` chain before `canvas.save_transform(...)`. Remove the per-paint `circle()` call and `Arc::new`. Uniform scaling leaves the conic weights unchanged, so the output is identical.

### A241-02 - crates/incular-widgets/src/overscroll_indicator.rs:778 (§23, medium)
Evidence:
```
let mut canvas = incular_rendering::Canvas::default();
let sides = [
```
Suggested fix: Glow paint runs whenever the retained builder rebuilds, which happens on every glow tick (`state.bump()` in the listeners), so `Canvas::default()` and `canvas.finish()` allocate a new DisplayList each frame during an overscroll animation even when both glows are idle (`opacity == 0`). Early-return from `GlowIndicatorState::paint` (or skip the `CustomPaint` node) when both controllers have zero glow opacity, and/or cache the last DisplayList on the state and rebuild only when a controller's paint inputs change (dirty flag). Confirm the rebuild frequency before changing it.

### A268-02 - crates/incular-widgets/src/scrolling/sliver_fade.rs:110 (§11, medium)
Evidence:
```
for child in &mut layout.children {
    child.widget = crate::Opacity::new(opacity, child.widget.clone()).into();
}
```
Suggested fix: Each layout allocates a new Opacity node (Rc WidgetAllocation) per visible child while the animation runs. Apply the opacity through the same retained wrap path the controlled variant uses (WidgetWrapRenderSliver with the wrap closure, lines 75-81), so the layer applies it without a fresh node per child per layout. The same pattern repeats in AnimatedOpacityRenderSliver::perform_layout at sliver_fade.rs:261-263; fix both together.

### A268-01 - crates/incular-widgets/src/scrolling/sliver_fade.rs:132 (§11, high)
Evidence:
```
for controller in self.animation.controllers() {
    controller.tick(now);
}
```
Suggested fix: Ticking runs every frame, but DoubleAnimation::controllers() (incular-animation/src/animations.rs:90) returns a Vec, so each tick allocates. Add a non-allocating visitor to the AnimationSource trait and DoubleAnimation, e.g. fn for_each_controller(&self, f: &mut dyn FnMut(&AnimationController)) (default impl forwards to controllers() for existing sources, overridden by the sources that own controllers), plus DoubleAnimation::tick_controllers(&self, now: Instant). Then here: self.animation.tick_controllers(now);

### A272-02 - crates/incular-widgets/src/spell_check.rs:157 (§2, low)
Evidence:
```
let mut corrected = Vec::new();
let mut offset: isize = 0;
let mut search_start = 0usize;
```
Suggested fix: Reserve the known upper bound: `let mut corrected = Vec::with_capacity(results.len());` (line 157). Same pattern for `let mut children = Vec::new();` at line 212 (subtrees_without_composing_region) and line 251 (subtrees_with_composing_region), where the bound is `spans.len() * 2 + 1` (each span plus at most one gap piece, plus a tail). These functions run in the text-field layout path that rebuilds the span tree.

### A272-01 - crates/incular-widgets/src/spell_check.rs:389 (§2, medium)
Evidence:
```
pub(crate) fn received(&self) -> Option<SpellCheckResults> {
.filter(|results| !results.suggestion_spans.is_empty())
.cloned()
```
Suggested fix: Return a borrow instead of cloning the whole SpellCheckResults (text String plus spans Vec) on every layout of a text field:

use std::cell::Ref;

pub(crate) fn received(&self) -> Option<Ref<'_, SpellCheckResults>> {
    Ref::filter_map(self.results.borrow(), |results| {
        results.as_ref().filter(|results| !results.suggestion_spans.is_empty())
    })
    .ok()
}

Caller text.rs:324 (`let Some(results) = session.received()`) and `&results` at text.rs:343 still compile via deref. Keep the Ref short-lived: the answer callback only runs asynchronously and calls borrow_mut, so no overlap occurs during layout. `results()` (line 398) is only used by the hidden test accessor and can stay as is.

### A279-02 - crates/incular-widgets/src/tree/focus.rs:396 (§2, high)
Evidence:
```
self.scrollable_viewport_chain(element)
.into_iter()
.find(|(_, _, viewport_axis, _, _)| *viewport_axis == axis)
```
Suggested fix: `scrollable_viewport_chain` (tree/interaction.rs:432) builds and returns a full `Vec` of cloned controllers and physics for every scroll ancestor, and this helper runs once per focusable candidate in `directional_focus_target` (line 313), i.e. one allocation and a full chain walk per element per key press. Make the chain a lazy iterator, e.g. `fn scrollable_viewports(&self, element) -> impl Iterator<Item = (...)> + '_` walking `self.parent(..)`, then here use `self.scrollable_viewports(element).find(|(_, _, a, _, _)| *a == axis).map(|(owner, ..)| owner)`. Alternatively walk ancestors and return the first matching scroll render without cloning. The lazy iterator also removes the clones in `scroll_nearest_viewport`.

### A279-01 - crates/incular-widgets/src/tree/focus.rs:608 (§2, high)
Evidence:
```
let focus_children: Vec<_> = match element.widget.kind() {
WidgetKind::IndexedStack { index, .. } => { element.children.get(*index).copied().into_iter().collect() }
_ => element.children.clone(),
```
Suggested fix: Borrow the child slice instead of allocating per node. `element` is borrowed from `self.elements` and `work` is a local, so no conflict:
`let focus_children: &[ElementId] = match element.widget.kind() {
    WidgetKind::IndexedStack { index, .. } => element.children.get(*index).map_or(&[], std::slice::from_ref),
    _ => &element.children,
};`
and loop with `for &child in focus_children.iter().rev() {` (the existing body is unchanged).

### A289-02 - crates/incular-widgets/src/tree/layout/behavior/sliver.rs:75 (§2, medium)
Evidence:
```
*last_published = Some(PublishedViewportAttempt {
size,
viewport_extent: viewport,
```
Suggested fix: Stop cloning the whole SliverViewportLayout (children Vec) on every shrink-wrap publication. Have publish_sliver_attempt store the layout once in `last_published` (move it in) and have callers read the current layout from `last_published` by reference, or return the layout only where the caller needs an owned value. Remove `layout: layout.clone()`.

### A289-01 - crates/incular-widgets/src/tree/layout/behavior/sliver.rs:101 (§2, high)
Evidence:
```
.render_live(id, "sliver viewport render must remain live")
.children
.clone();
```
Suggested fix: Drop the Vec clone: RenderObjectId is Copy, so iterate by index. Replace the `let materialized = ...clone();` binding with `let child_count = self.render_live(id, "sliver viewport render must remain live").children.len().min(sliver_layout.children.len());` and in the loop read `let child = self.render_live(id, "sliver viewport render must remain live").children[index];` paired with `let layout = &sliver_layout.children[index];` (same truncation as the original zip).


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
