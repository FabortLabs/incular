# Fix brief incular-widgets-27

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree.rs`, `crates/incular-widgets/src/tree/departing.rs`, `crates/incular-widgets/src/tree/layout/behavior/core.rs`, `crates/incular-widgets/src/tree/rendering.rs`, `crates/incular-widgets/src/tree/rendering/lowering/scrolling.rs`, `crates/incular-widgets/src/tree/rendering/lowering/visual.rs`, `crates/incular-widgets/src/tree/specs.rs`, `crates/incular-widgets/src/tree/values.rs`, `crates/incular-widgets/src/tree/widget/mod.rs`, `crates/incular-widgets/src/animated_size.rs`. Other agents are editing other files at
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
### A274-01 - crates/incular-widgets/src/tree.rs:1422 (§2, medium)
Evidence:
```
let mut work = vec![self];
while let Some(member) = work.pop() {
    match member {
```
Suggested fix: Replace the iterative body with a recursive search that allocates nothing. Pre-order, first-member-first order is preserved:
    fn first_candidate(&self) -> Option<&FocusCandidate> {
        match self {
            Self::Candidate(candidate) => Some(candidate),
            Self::Group(group) => group.members.iter().find_map(Self::first_candidate),
        }
    }
The callers representative_bounds, representative_order and representative_sequence run once per comparison inside sort_by and sort_by_key in sort_focus_members, so this currently heap-allocates a Vec on every comparison of a focus sort.

### A275-02 - crates/incular-widgets/src/tree/departing.rs:104 (§23, medium)
Evidence:
```
let position = previous[..index]
.iter()
.rev().find_map(|before| next.iter().position(|child| child == before))
```
Suggested fix: For each kept id the code runs a linear scan of next per preceding sibling, which is O(kept * previous * next) per reconciliation and grows with list length. Resolve anchors with a position lookup over next (for example a HashMap built from next only when kept is non-empty, or a position map kept in step with the inserts). The lookup must still see kept siblings already inserted earlier in this loop, so that consecutive departing children keep their relative order. Add a tests/ case for a run of adjacent departing children.

### A288-01 - crates/incular-widgets/src/tree/layout/behavior/core.rs:61 (§2, high)
Evidence:
```
(constraints.constrain(child_size), vec![Offset::ZERO])
let node = self.render_live_mut(id, "retained render must remain live");
(size, offsets.into())
```
Suggested fix: Use the inline single-child form used by every other arm and drop the Vec: in the `if let Some(&child)` branch return `(constraints.constrain(child_size), ChildOffsets::One(Offset::ZERO))`; in the else branch return `(constraints.constrain(Size::ZERO), ChildOffsets::default())`; then bind `let (size, offsets) = ...;` as `(size, offsets)` with no `.into()` (or restructure the match arm so the tuple is returned directly).

### A298-01 - crates/incular-widgets/src/tree/rendering.rs:34 (§2, high)
Evidence:
```
let mut resolved = context
.depend::<TextStyle>()
.map_or(std::borrow::Cow::Borrowed(style), |default| {
```
Suggested fix: Use the shared read so the ambient TextStyle is not deep-cloned for every text element: `let mut resolved = context.depend_shared::<TextStyle>().map_or(std::borrow::Cow::Borrowed(style), |default| { std::borrow::Cow::Owned(default.merge(style)) });`. `depend_shared` returns Option<Rc<TextStyle>>; `TextStyle::merge(&self, &Self)` auto-derefs through the Rc, so the body is unchanged.

### A298-03 - crates/incular-widgets/src/tree/rendering/lowering/scrolling.rs:90 (§2, medium)
Evidence:
```
context
.depend::<crate::ScrollBehavior>()
.unwrap_or_default()
```
Suggested fix: Avoid deep-cloning the ambient ScrollBehavior for every Scroll/SliverViewport lowering: read it with `context.depend_shared::<crate::ScrollBehavior>()` and borrow it (`&ScrollBehavior`) for `resolve()` and `inherited_physics`, cloning only where ownership is required. Verify `ScrollBehavior::resolve`'s receiver before changing; if it takes `&self` the clone can go entirely.

### A299-01 - crates/incular-widgets/src/tree/rendering/lowering/visual.rs:117 (§2, medium)
Evidence:
```
.collect::<Vec<_>>()
.into()
```
Suggested fix: Collect straight into the target type and drop the intermediate Vec (one allocation instead of two): `spans: spans.as_ref().map(|runs| runs.iter().map(|run| StyledTextRun { text: run.text.clone(), style: resolve_text_style(&run.style, context).scaled(*text_scaler), range: run.range.clone() }).collect::<Arc<[_]>>()),`

### A303-01 - crates/incular-widgets/src/tree/specs.rs:314 (§8, low)
Evidence:
```
pub(crate) has_callback: bool,
pub(crate) child: Option<Widget>,
```
Suggested fix: Drop the redundant `has_callback` flag and derive presence from the bound action: make `action` an `Option<ActionId>` (set to `Some(id)` in `bind_callbacks`, `None` otherwise) and replace `spec.has_callback` with `spec.action.is_some()` at semantics.rs:690 and :1338 (the `spec.action != ActionId(0)` check there becomes `spec.action.is_some()`). Update constructors.rs:424, :468, :499 accordingly. Alternatively keep the bool and document why it differs from `action != ActionId(0)`.

### A306-01 - crates/incular-widgets/src/tree/values.rs:757 (§3, low)
Evidence:
```
let mut matrix = [0.; 20];
        for index in 0..20 {
            matrix[index] = from[index] + (to[index] - from[index]) * t;
```
Suggested fix: let matrix = std::array::from_fn(|index| from[index] + (to[index] - from[index]) * t);
        self.set_matrix(matrix)

### A309-01 - crates/incular-widgets/src/tree/widget/mod.rs:105 (§8, low)
Evidence:
```
match Rc::try_unwrap(node) {
    Ok(node) => drop(node),
    Err(node) => drop(node),
}
```
Suggested fix: let next = queue.borrow_mut().pending.pop();
let Some(node) = next else { break; };
// Dropping the Rc releases the allocation when it is unique, and the child Widget
// drops then enqueue into this active queue instead of recursing.
drop(node);
(The try_unwrap match is removed; both arms are identical `drop(node)`.)

### U123-03 - crates/incular-widgets/src/animated_size.rs:286 (§12, medium)
Evidence:
```
                            && let Some(on_end) = state
                                .config
                                .borrow()
```
Suggested fix: Clone the callback in a `let` statement so the `RefCell` borrow ends before the call, then run it outside the borrow:
    let on_end = state.config.borrow().as_ref().and_then(|config| config.on_end.clone());
    if status == AnimationStatus::Completed
        && let Some(on_end) = on_end
    { on_end(); }
(Restructure the existing `if status == ... && let ...` block accordingly.) In edition 2024 the `if let` scrutinee temporary, the `Ref` from `borrow()`, stays alive through the `then` block, so `on_end()` currently runs while `SizeState.config` is borrowed.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
