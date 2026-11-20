# Fix brief incular-widgets-18

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/raw_tooltip.rs`, `crates/incular-widgets/src/scrolling/sliver_descriptors.rs`, `crates/incular-widgets/src/tree/layout/behavior/scrolling.rs`. Other agents are editing other files at
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
### A251-01 - crates/incular-widgets/src/raw_tooltip.rs:644 (§2, high)
Evidence:
```
let mut controllers = Vec::new();
controllers.push(Self { state });
for controller in controllers {
```
Suggested fix: Do not collect into a Vec. hide_at() never touches OPEN_TOOLTIPS and invokes no callbacks, so the registry can be walked in place while borrowed. Replace the body of dismiss_other_hovered with: OPEN_TOOLTIPS.with(|open| { let mut open = open.borrow_mut(); open.retain(|weak| weak.strong_count() != 0); for weak in open.iter() { let Some(state) = weak.upgrade() else { continue }; if Rc::ptr_eq(&state, &self.state) { continue; } let controller = Self { state }; if !controller.is_kept_open() { let _ = controller.hide_at(now); } } }); The same helper shape can replace the collecting block in dismiss_all_at (lines 535-546). Behavior is unchanged: the self-skip, keep-open check and prune semantics are identical, and no per-event Vec is allocated.

### A252-03 - crates/incular-widgets/src/raw_tooltip.rs:791 (§8, low)
Evidence:
```
semantics_tooltip: Option<String>,
semantics_explicit: bool,
```
Suggested fix: Encode the three states (no explicit semantics, explicit clear, explicit text) as one field: `semantics_tooltip: Option<Option<String>>`, or as a small private enum `SemanticsOverride { Inherit, Clear, Set(String) }`. Update clear_semantics_tooltip/semantics_tooltip and semantic_value to match on it.

### A252-01 - crates/incular-widgets/src/raw_tooltip.rs:1180 (§2, medium)
Evidence:
```
.is_some_and(|message| !message.plain_text().is_empty())
```
Suggested fix: Add an allocation-free emptiness check to RichText in incular-text (e.g. `pub fn is_plain_text_empty(&self) -> bool` that walks the TextSpan tree and returns false at the first non-empty text run) and use it here: `.is_some_and(|message| !message.is_plain_text_empty())`. Same helper can replace `plain_text().is_empty()` in semantic_value if desired.

### A252-04 - crates/incular-widgets/src/raw_tooltip.rs:1322 (§2, low)
Evidence:
```
let callbacks = callbacks.clone();
```
Suggested fix: Drop the redundant outer clone: `callbacks` is not used after the factory is built. Write `let factory = GestureRecognizerFactoryWithHandlers::new(
    move || PointerGestureRecognizer::new(callbacks.clone()),
    |_recognizer| {},
);` (the per-recognizer clone stays because each recognizer takes its own GestureCallbacks).

### A266-03 - crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1309 (§2, medium)
Evidence:
```
            children: (!self.children.is_empty()).then(|| Rc::new(self.children.clone())),
```
Suggested fix: Every `create_render_sliver` call clones the whole `Vec<Widget>` into a new `Rc`. Store the children once at construction as `children: Rc<[Widget]>` (built in `new`/`typed_builder` transform via `Rc::from(vec)`), then here use `children: (!self.children.is_empty()).then(|| Rc::clone(&self.children))`. This keeps the per-sliver render setup allocation-free apart from the Rc clone. Update the field type and the `Some`/`is_empty` checks at lines 1306-1309 accordingly.

### A266-01 - crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1476 (§12, medium)
Evidence:
```
    children: Vec<(RefCell<Box<dyn RenderSliver>>, usize)>,
```
Suggested fix: Store the children without interior mutability: `children: Vec<(Box<dyn RenderSliver>, usize)>` (drop the `RefCell::new` in `new`, mapping to `(child, flex.max(1))`). Every method that mutates a child already takes `&mut self` (`perform_layout`, `set_child_extent`, `tick`, `set_reduced_motion`), so use `self.children.iter_mut()` / `.get_mut(lane)` and call `sliver.perform_layout(..)` directly instead of `sliver.borrow_mut().perform_layout(..)`. The `&self` methods (`scroll_layout_dependency`, `revision`, `is_animating`) use `sliver.scroll_layout_dependency()` etc. with no borrow.

### A266-02 - crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1509 (§2, medium)
Evidence:
```
        let mut children = Vec::new();
        for (lane, (sliver, flex)) in self.children.iter().enumerate() {
```
Suggested fix: Per-frame Vec growth in layout. `SliverLayout` owns `children` by value, so the buffer cannot simply be hoisted without changing the RenderSliver layout API. At minimum pre-size it: `let mut children = Vec::with_capacity(self.children.len());` (lower bound, one child per lane). A reusable buffer would need `SliverLayout` to borrow or return the buffer to the caller; decide that together with the layout contract in system-design/ARCHITECTURE.md.

### A267-02 - crates/incular-widgets/src/scrolling/sliver_descriptors.rs:2072 (§2, low)
Evidence:
```
Rc::new(move |widget| {
    crate::layout::Visibility::new(widget)
...
(self.wrap())(self.sliver.build(controller))
```
Suggested fix: Move the closure body of `wrap()` into a plain function, e.g. `fn wrap_visibility(flags: VisibilityFlags, widget: Widget) -> Widget` where `VisibilityFlags` is a Copy struct of the six values (or a tuple). Call it directly in build: `wrap_visibility(self.flags(), self.sliver.build(controller))`. Keep the `Rc<dyn Fn>` only in create_render_sliver, where the closure must be stored in the render sliver.

### A288-02 - crates/incular-widgets/src/tree/layout/behavior/scrolling.rs:60 (§2, high)
Evidence:
```
let size = constraints.constrain(
self.render_live(child, "retained render must remain live")
(size, vec![Offset::ZERO])
```
Suggested fix: Return `(size, ChildOffsets::One(Offset::ZERO))` instead of `(size, vec![Offset::ZERO])`, and `(constraints.constrain(Size::ZERO), ChildOffsets::default())` instead of `Vec::new()` in the else branch (line 62); drop the trailing `offsets.into()` on line 83.

### A288-04 - crates/incular-widgets/src/tree/layout/behavior/scrolling.rs:96 (§2, medium)
Evidence:
```
.map(|child| {
(
.collect::<Vec<_>>(),
```
Suggested fix: The Vec exists only to release the `wheel_state_live(id)` borrow before `self.layout_render`. Avoid the per-layout allocation by iterating from a reusable scratch buffer owned by the tree or wheel state (cleared and refilled each layout), or by re-reading the child rect by index inside the loop so no placements Vec is built. Keep `Vec::with_capacity` only for the output offsets.

### A288-03 - crates/incular-widgets/src/tree/layout/behavior/scrolling.rs:148 (§2, high)
Evidence:
```
vec![Offset::new(
((size.width - child_width) * 0.5).max(0.0),
(size.height - child_height).max(0.0),
```
Suggested fix: Build a single `ChildOffsets::One(Offset::new(((size.width - child_width) * 0.5).max(0.0), (size.height - child_height).max(0.0)))` for the child branch and `ChildOffsets::default()` in the else branch (line 153), instead of `vec![...]` / `Vec::new()` and the `offsets.into()` on line 155.

### A288-05 - crates/incular-widgets/src/tree/layout/behavior/scrolling.rs:184 (§2, medium)
Evidence:
```
.map(|child| {
(child.constraints.as_box_constraints(), child.paint_offset)
.collect::<Vec<_>>(),
```
Suggested fix: Same borrow-driven collect as the list wheel arm: avoid the intermediate `Vec<(BoxConstraints, Offset)>` by reusing a scratch buffer on the two-dimensional state/tree or by indexing the layout's children inside the loop, re-borrowing the state per child. Output `offsets` Vec stays.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
