# Fix brief incular-widgets-20

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs`, `crates/incular-widgets/src/advanced_scrolling/wheel.rs`, `crates/incular-widgets/src/provider_image.rs`, `crates/incular-widgets/src/render_object/layers.rs`. Other agents are editing other files at
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
### A192-02 - crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:883 (§10, medium)
Evidence:
```
if !self.cache.contains_key(&vicinity)
    && let Some(child) = self.delegate.child_at(vicinity)
{
    self.cache.insert(vicinity, child);
```
Suggested fix: Use the entry API so each cell does one lookup instead of contains_key + insert + get: match self.cache.entry(vicinity) { Entry::Occupied(e) => e.into_mut(), Entry::Vacant(e) => match self.delegate.child_at(vicinity) { Some(c) => e.insert(c), None => continue } }, then clone the child once. self.cache and self.delegate are disjoint fields, so the borrows are fine.

### A192-03 - crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:891 (§11, medium)
Evidence:
```
let row_extent = extent_at(&self.rows, y_index);
let column_extent = extent_at(&self.columns, x_index);
```
Suggested fix: row_extent depends only on y_index, but it is recomputed (two offset_for_index lookups) for every cell in the inner x loop. Move `let row_extent = extent_at(&self.rows, y_index);` to just after the `for y_index in row_range.clone() {` line and keep only `let column_extent = extent_at(&self.columns, x_index);` inside the inner loop (or precompute column extents once per layout into a scratch buffer).

### A192-04 - crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:990 (§4, medium)
Evidence:
```
children.sort_by(|left, right| match self.main_axis {
    Axis::Horizontal => left
        .vicinity
```
Suggested fix: Change to children.sort_unstable_by(|left, right| match self.main_axis { ... }) with the same comparator. Every child has a unique vicinity (cache is keyed by it and each cell is visited once), so stability is irrelevant, and the stable sort allocates a scratch buffer on every layout for non-trivial lengths. Optionally skip the sort for the Vertical main axis, where the row-major raw order is already the required order.

### A193-06 - crates/incular-widgets/src/advanced_scrolling/wheel.rs:445 (§11, medium)
Evidence:
```
let Some(local) = child.transform.inverse_transform_point(point) else {
    continue;
};
```
Suggested fix: hit_test calls inverse_transform_point per visible child per pointer event, and each call runs the full 4x4 Gauss-Jordan `inverse()` (line 220) on an immutable transform fixed at layout time. Compute the inverse once in layout_with_offset and store it on WheelChildLayout (e.g. `pub(crate) inverse: Option<WheelMatrix>`, set from `transform.inverse()`), then have hit_test apply that stored inverse directly.

### A193-05 - crates/incular-widgets/src/advanced_scrolling/wheel.rs:783 (§2, medium)
Evidence:
```
let mut children = Vec::new();
for index in target_range.clone() {
    let Some(child) = self.delegate.child_at(index) else {
```
Suggested fix: The number of children is known before the loop (target_range is computed above and cloned per iteration). Replace with `let mut children = Vec::with_capacity(target_range.len());` so the per-frame layout pass does not regrow the Vec.

### A193-07 - crates/incular-widgets/src/advanced_scrolling/wheel.rs:922 (§2, medium)
Evidence:
```
delegate: self.delegate.clone(),
on_selected_item_changed: self.on_selected_item_changed.clone(),
...
Self::Children(children) => Self::Children(children.clone()),
```
Suggested fix: instantiate() (called from update_runtime, which replaces the whole viewport) clones WheelChildDelegate::Children(Vec<T>), i.e. every eager child in the list, each time. Store the eager list as `Rc<[T]>` (or `Arc<[T]>`) in the Children variant so both the Clone impl (line 43) and instantiate become refcount bumps. Public constructor `children(Vec<T>)` converts with `Rc::from(vec)`. Frequency depends on how often update_runtime runs; confirm before treating as high.

### A248-08 - crates/incular-widgets/src/provider_image.rs:179 (§11, high)
Evidence:
```
if let Some(locale) = context.locales().into_iter().next() {
configuration = configuration.locale(locale);
}
```
Suggested fix: BuildContext::locales() (tree/context.rs:231) returns a freshly collected Vec on every call, and this runs on every build of every ProviderImage. Add a `BuildContext::primary_locale(&self) -> Option<Locale>` that reads only the first entry, and write: `if let Some(locale) = context.primary_locale() {`.

### A248-09 - crates/incular-widgets/src/provider_image.rs:247 (§2, medium)
Evidence:
```
let key = (config.provider.clone(), configuration);
let mut resolved = self.resolved.borrow_mut();
let mut stream = self.stream.borrow_mut();
```
Suggested fix: stream() runs on every build and always builds the key (cloning the provider Rc and moving in a configuration whose Option<Locale> may own Strings) before comparing. Compare against the stored key by reference first, and build the owned key only on a miss: `if let (Some((provider, cfg)), Some(s)) = (resolved.as_ref(), stream.as_ref()) && provider == &config.provider && *cfg == configuration { return s.clone(); }` then construct `(config.provider.clone(), configuration)` after the early return.

### A248-10 - crates/incular-widgets/src/provider_image.rs:304 (§2, medium)
Evidence:
```
} else if let Some(label) = &config.semantic_label {
result = Semantics::new(result)
.label(label.clone())
```
Suggested fix: Every build with a semantic label allocates a new String via label.clone(). Store the label as a shared immutable string (`semantic_label: Option<Arc<str>>` or Rc<str>) and have Semantics::label (widgets semantics.rs:77) accept that shared type, so the per-build step is a refcount bump. Keep Semantics::label(impl Into<String>) only if no other caller needs the change.

### A254-01 - crates/incular-widgets/src/render_object/layers.rs:122 (§2, high)
Evidence:
```
applied: Box<std::cell::RefCell<ClipApplied>>,
```
Suggested fix: Change the field to `applied: Rc<std::cell::RefCell<ClipApplied>>,` (add `use std::rc::Rc;`). Every `applied: Default::default()` still compiles. `RenderLayers` is cloned on every kind change (tree/reconciliation.rs:666 and :678, `.layers.clone()`), and the derived Clone currently allocates a new Box and deep-copies ClipApplied (including its Option<RenderKind> and delegate) each time. With Rc the clone is a refcount bump that shares the same state. This is safe because the clone is written back over the original (reconciliation.rs:671) and only one copy stays live.

### A254-03 - crates/incular-widgets/src/render_object/layers.rs:387 (§3, low)
Evidence:
```
let mut layers = picture.into_iter().collect::<Vec<_>>();
layers.extend(focus_picture);
```
Suggested fix: Replace the two statements with `compositor.set_children(root, picture.into_iter().chain(focus_picture).collect());` (the `_` arm of `RenderLayers::create`). This removes the intermediate `collect` and the extend. It runs only on mount and structure reconcile, not per frame.

### A254-02 - crates/incular-widgets/src/render_object/layers.rs:941 (§2, medium)
Evidence:
```
let Some(geometry) = clip_geometry(kind, size) else {
    return;
};
```
Suggested fix: Move the cache check in `apply_clip` ahead of geometry construction. Compute `size_changed`, `delegate_kept` and `inputs_kept` first, return early when the cache holds, and only then call `clip_geometry(kind, size)`. The current order runs the user's CustomClipper (`clipper.clip(size)`) and, for ClipPath with a non-path clipper, `rect_path` (a new Path builder plus Arc allocation) on every layout and kind update, and then throws the result away when the cache hits. `apply_clip` runs from update_layout_geometry (tree/layout.rs:1104) for every laid-out clip node.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
