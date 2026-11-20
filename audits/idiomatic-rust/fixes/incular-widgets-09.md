# Fix brief incular-widgets-09

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/tree/painting.rs`. Other agents are editing other files at
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
### A290-04 - crates/incular-widgets/src/tree/painting.rs:545 (§2, medium)
Evidence:
```
result.content_hit |= child_result.content_hit;
result.elements.extend(child_result.elements);
if child_target.is_some() {
```
Suggested fix: Each visited node with a raw annotation allocates its own `elements` Vec and the parent then copies it, so a pointer event allocates once per annotated ancestor on the path. Pass a single `&mut Vec<ElementId>` sink down `collect_pointer_hit_render` / `collect_pointer_hit_children` (children push into the sink before the parent pushes its own element, so order is unchanged) and truncate the sink to its previous length when a non-content-hit overlay result is discarded (painting.rs:379-383). Reuse the sink's capacity across events via a scratch buffer.

### A290-05 - crates/incular-widgets/src/tree/painting.rs:706 (§2, medium)
Evidence:
```
let Some(node) = self.semantics.node(id).cloned() else {
continue;
};
```
Suggested fix: Clones a whole SemanticNode (strings, actions, children) per node per paint while the SemanticsDebugger is on, only to dodge the `&mut self` borrow used by `paint_semantics_debug_label`. Borrow instead and finish with the borrow before the label call: `let Some(node) = self.semantics.node(id) else { continue; };`, copy `bounds`, compute `message = semantics_debug_message(node)`, push `node.children` onto `stack`, then call `self.paint_semantics_debug_label(...)` after the last use of `node`.

### A291-01 - crates/incular-widgets/src/tree/painting.rs:706 (§2, medium)
Evidence:
```
let Some(node) = self.semantics.node(id).cloned() else {
 continue;
 };
```
Suggested fix: Clone nothing: read the fields inside a borrow scope and drop the borrow before the &mut self call. `let (bounds, message) = { let Some(node) = self.semantics.node(id) else { continue; }; for child in node.children.iter().rev() { stack.push((*child, depth.saturating_add(1))); } (node.bounds, semantics_debug_message(node)) };` then paint with `bounds` and `message` (keep the painted += 1 and the colors lookup as they are). Removes a full SemanticNode clone (children Vec plus label/value/description Strings) per painted node per frame while the semantics debugger is on. Child push order is unchanged because painting never touches the stack.

### A291-05 - crates/incular-widgets/src/tree/painting.rs:916 (§11, low)
Evidence:
```
let kind = self
 .element_for_render(id)
 .and_then(|element| self.elements.get(element.0))
```
Suggested fix: Look the element's widget kind up once. hit_test_render already performs the same element_for_render plus elements.get lookup at lines 846-849 for the IgnorePointer/AbsorbPointer match. Hoist that into `let kind = ...;` before the match and reuse it in `self_hit_after_children(kind)` at line 920 instead of repeating the lookup after the child walk. Both lookups read the same &self, so the value is identical.

### A291-03 - crates/incular-widgets/src/tree/painting.rs:1011 (§11, high)
Evidence:
```
slots
 .into_iter()
 .filter_map(|slot| node.children.get(slot).copied())
 .collect::<Vec<_>>()
```
Suggested fix: Stop rebuilding the paint order on every pointer event. Compute the slot order once where sync_render_children orders the painted layers (or when the viewport's paint order or child scopes change, guarded by a generation), store it with the viewport's render state as a Box<[RenderObjectId]>, and return std::borrow::Cow::Borrowed(&stored) here. The current path allocates twice per hit-tested viewport: sliver_paint_slots (scrolling/geometry.rs:74, a Vec<usize>) and this collect. The allocation happens only when the scopes are mixed and paint_order is not LastIsTop, so confidence in the hot-path cost is medium.

### A290-02 - crates/incular-widgets/src/tree/painting.rs:1014 (§2, medium)
Evidence:
```
.filter_map(|slot| node.children.get(slot).copied())
.collect::<Vec<_>>()
.into()
```
Suggested fix: This runs per pointer event for every sliver viewport whose children use mixed paint scopes. `sliver_paint_slots` already allocates a Vec of slots (scrolling/geometry.rs:63), so this is a second allocation. Compute the reordered child list in a reused buffer on the element/viewport (cleared and refilled only when the children change), or have `hit_children` return the slot order and iterate `node.children` through it without collecting, so the hit path does not allocate per event.

### A291-02 - crates/incular-widgets/src/tree/painting.rs:1154 (§5, medium)
Evidence:
```
let mut fields = vec![semantic_role_name(node.role).to_owned()];
if let Some(label) = node.label.as_deref().filter(|label| !label.is_empty()) {
 fields.push(format!("label={:?}", bounded_debug_text(label, 120)));
```
Suggested fix: Build the debug message into one String owned by the debugger pass, hoisted above the paint loop as `let mut message = String::new();` and `message.clear()` per node. Change semantics_debug_message(node, out: &mut String) to `out.push_str(semantic_role_name(node.role));` and `write!(out, ' label={:?}', ...)` in place of format!. Push separators with out.push(' ') rather than Vec<String> plus fields.join(" "). Make bounded_debug_text write into `out` (or return &str slices via char_indices) rather than calling to_owned() on short text. Today each painted node allocates a Vec, several Strings, and the join result, all per frame while the debugger is on.

### A290-06 - crates/incular-widgets/src/tree/painting.rs:1155 (§5, low)
Evidence:
```
let mut fields = vec![semantic_role_name(node.role).to_owned()];
if let Some(label) = node.label.as_deref().filter(|label| !label.is_empty()) {
fields.push(format!("label={:?}", bounded_debug_text(label, 120)));
```
Suggested fix: Opt-in SemanticsDebugger only. Build the label with a single `String` and `write!` into it (`use std::fmt::Write`) instead of a Vec of Strings joined at the end, and reuse one buffer across nodes so the debugger does not allocate per node per paint.

### A291-04 - crates/incular-widgets/src/tree/painting.rs:1300 (§11, high)
Evidence:
```
let mut moved = (**run).clone();
 moved.origin.x += origin.x;
 Arc::new(moved)
```
Suggested fix: Do not allocate a moved Arc per glyph run per paint. Translate the whole line once with a transform, as paint_semantics_debug_label already does for its GlyphRun commands: when origin != Offset::ZERO, push one PaintCommand::PushTransform { transform: CoreTransform::translation(origin) } before the line's runs, push each run as run.clone() (an Arc clone), then one PopTransform. Confirm the renderer applies the transform to GlyphRun origins the same way before merging; painted_bounds already treats glyph runs as inside the box, so its result is unchanged.

### A290-01 - crates/incular-widgets/src/tree/painting.rs:1303 (§2, high)
Evidence:
```
let mut moved = (**run).clone();
moved.origin.x += origin.x;
Arc::new(moved)
```
Suggested fix: Do not deep-clone the glyph run per paint. Keep `run.clone()` (an Arc bump) and emit the line origin as a translation instead: push `PaintCommand::PushTransform { transform: CoreTransform::translation(origin) }` before the runs of a line whose origin is non-zero and `PaintCommand::PopTransform` after them, as `paint_semantics_debug_label` already does. Then the `if origin == Offset::ZERO` branch and the per-run `Arc::new` both go away.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
