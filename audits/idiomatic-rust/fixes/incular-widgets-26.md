# Fix brief incular-widgets-26

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/fade_in_image.rs`, `crates/incular-widgets/src/layout/basic/painting.rs`, `crates/incular-widgets/src/notification.rs`, `crates/incular-widgets/src/painting_effects/shape_decoration.rs`, `crates/incular-widgets/src/raw_input.rs`, `crates/incular-widgets/src/raw_menu_anchor.rs`, `crates/incular-widgets/src/render_object/mod.rs`, `crates/incular-widgets/src/scroll_aware_image_provider.rs`, `crates/incular-widgets/src/scrolling/geometry.rs`, `crates/incular-widgets/src/scrolling/scroll_views.rs`, `crates/incular-widgets/src/size_changed.rs`, `crates/incular-widgets/src/transient.rs`. Other agents are editing other files at
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
### A221-04 - crates/incular-widgets/src/fade_in_image.rs:433 (§11, medium)
Evidence:
```
move |state, _, _| {
    ...
    state.update(&config, loaded, synchronous);
    state.build(config.placeholder(), target.clone())
```
Suggested fix: `config.placeholder()` is static for a given config, but it is rebuilt on every fade tick (the controller listener bumps the revision each frame, which reruns this closure). It allocates a new image descriptor, clones the handle or provider, and recomputes options each frame. Build it once in `fade()` before `Widget::retained_builder` (or cache it in `FadeState` alongside `config`, rebuilt only in `update` when the config changes) and pass `placeholder.clone()` (an Rc clone) to `state.build`.

### A233-01 - crates/incular-widgets/src/layout/basic/painting.rs:360 (§3, low)
Evidence:
```
.collect::<Vec<_>>();
Stack::new(regions).clip_behavior(Clip::None).into()
```
Suggested fix: Drop the `let regions = ...` binding and the `.collect::<Vec<_>>()`; pass the iterator straight into the constructor: `Stack::new(builder(constraints.biggest()).into_iter().map(|region| { ... node.with_key(key)... })).clip_behavior(Clip::None).into()`. Stack::new already accepts `impl IntoIterator` and collects once itself, so the intermediate Vec is redundant.

### A240-02 - crates/incular-widgets/src/notification.rs:102 (§2, medium)
Evidence:
```
*node.handlers.borrow_mut() = handlers.clone();
```
Suggested fix: Convert once outside the retained builder closure, in listen(): let handlers: Rc<[NotificationHandler]> = handlers.into(); before Widget::retained_builder. Then the per-build assignment becomes *node.handlers.borrow_mut() = handlers.clone(); (an Rc clone, no Vec clone per build). Pairs with the dispatch change on line 73, which uses the same Rc<[NotificationHandler]> field type.

### A246-04 - crates/incular-widgets/src/painting_effects/shape_decoration.rs:166 (§2, medium)
Evidence:
```
let mut layers: Vec<Widget> = decoration
.shadows
.iter()
```
Suggested fix: Reserve for every layer up front and extend instead of collect-then-push, which reallocates on the first push: `let mut layers: Vec<Widget> = Vec::with_capacity(decoration.shadows.len() + 3); layers.extend(decoration.shadows.iter().map(|shadow| { /* same body */ }));`

### A249-01 - crates/incular-widgets/src/raw_input.rs:1040 (§2, medium)
Evidence:
```
Some((
                listener,
                Box::new(move || callback(event)) as Box<dyn FnOnce()>,
```
Suggested fix: Return the event alongside the callback instead of wrapping it in a second box. Change `SignalWinner` (line 956) to `(Option<crate::tree::ElementId>, Box<dyn FnOnce(RawPointerEvent)>, RawPointerEvent)`, make `finish()` return `Some((listener, callback, event))` (drop the `Box::new(move || ...)` wrapper), and have the caller invoke `callback(event)`. This leaves one allocation per claimed pointer signal (the one in `register`, line 1005) instead of two.

### A250-01 - crates/incular-widgets/src/raw_menu_anchor.rs:109 (§2, high)
Evidence:
```
self.live_children().iter().any(|child| child.is_open())
```
Suggested fix: Avoid the Vec in live_children() on this path: prune in place, then iterate without collecting. Replace the group branch of is_open with: `if self.is_group { self.children.borrow_mut().retain(|child| child.strong_count() > 0); self.children.borrow().iter().filter_map(Weak::upgrade).any(|child| child.is_open()) } else { self.open.get() }`. Also reorder the call in build (line 486) to `self.consume_outside_taps && root.is_open()` so is_open is not evaluated when consume_outside_taps is false. Keep live_children only where the collected Vec is needed (close_children, which re-enters close on children).

### A255-01 - crates/incular-widgets/src/render_object/mod.rs:398 (§18, low)
Evidence:
```
#[allow(non_upper_case_globals)]
impl SemanticsDirty {
```
Suggested fix: #[expect(non_upper_case_globals, reason = "bit-flag constants read as enum-like variant names")]
impl SemanticsDirty {

### A256-01 - crates/incular-widgets/src/scroll_aware_image_provider.rs:37 (§18, low)
Evidence:
```
#[allow(clippy::cast_possible_truncation)]
let scale = context.scale_factor() as f32;
```
Suggested fix: #[expect(clippy::cast_possible_truncation, reason = "the window scale factor is f64 but physical pixel sizes are f32")]

### A257-01 - crates/incular-widgets/src/scrolling/geometry.rs:74 (§2, high)
Evidence:
```
let mut slots: Vec<usize> = (0..ids.len()).collect();
slots.sort_by_key(|slot| std::cmp::Reverse(ids[*slot].scope()));
Some(slots)
```
Suggested fix: Stop returning a fresh Vec per call. Change the signature to fill a caller-owned scratch buffer: pub(crate) fn sliver_paint_slots(ids: &[SliverChildId], order: SliverPaintOrder, slots: &mut Vec<usize>) -> bool, which clears slots, pushes 0..ids.len(), stable-sorts by Reverse(scope), and returns false when no reordering is needed. Keep sort_by_key (stable), because children of one sliver must keep their relative order. Store the buffer on the viewport element or in the tree so the paint path (tree/painting.rs:1005-1015, which also collects a second Vec) and reconciliation (tree/reconciliation.rs:2319) reuse it. Alternatively, iterate distinct scopes in descending order and yield matching slots without any Vec.

### A263-01 - crates/incular-widgets/src/scrolling/scroll_views.rs:128 (§8, low)
Evidence:
```
pub(super) fn inherit_physics(widget: Widget, inherit: Option<bool>) -> Widget {
    match inherit {
        Some(page) => widget.inherit_scroll_physics(page),
```
Suggested fix: Replace the Option<bool> encoding (None / Some(false) = scroll physics / Some(true) = page physics) with an enum, e.g. `#[derive(Clone, Copy)] enum InheritedPhysics { Scroll, Page }` and `fn inherit_physics(widget: Widget, inherit: Option<InheritedPhysics>) -> Widget { match inherit { Some(InheritedPhysics::Page) => widget.inherit_scroll_physics(true), Some(InheritedPhysics::Scroll) => widget.inherit_scroll_physics(false), None => widget } }`. Update the call sites at lines 142, 635, 977 and 1499 (`then_some(false)` becomes `then_some(InheritedPhysics::Scroll)`, `then_some(value.page_snapping)` becomes a `page_snapping`-selected variant).

### A271-04 - crates/incular-widgets/src/size_changed.rs:61 (§12, low)
Evidence:
```
let value = RefCell::new(value);
Widget::retained_builder(
    |_| LastSize(Rc::new(Cell::new(None))),
```
Suggested fix: Drop the RefCell: retained_builder's build closure is `Fn` and only reads `value`, so capture it directly with `move |state, context, _| { ... value.child.clone() ... }` and remove the `let value = value.borrow();` line (line 65). The RefCell adds a runtime borrow check on every build with no mutation to justify it.

### A272-03 - crates/incular-widgets/src/transient.rs:10 (§26, low)
Evidence:
```
use incular_core::{Offset, Rect, Size};
use incular_core::{finite_non_negative, finite_or_zero};
```
Suggested fix: Merge the two imports from the same crate into one line: `use incular_core::{Offset, Rect, Size, finite_non_negative, finite_or_zero};`. Readability only.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
