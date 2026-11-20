# Fix brief incular-widgets-21

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/scrolling/reorderable.rs`, `crates/incular-widgets/src/selection.rs`, `crates/incular-widgets/src/tree/descriptors.rs`, `crates/incular-widgets/src/tree/layout/behavior/text.rs`. Other agents are editing other files at
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
### A262-03 - crates/incular-widgets/src/scrolling/reorderable.rs:30 (§2, medium)
Evidence:
```
Key::String(format!(
        "{LISTENER_MARKER_PREFIX}{}:{}:{}",
```
Suggested fix: Each listener build allocates and marker_spec later re-parses this string per row. Replace the string-encoded marker with a non-allocating key (e.g. a packed integer key variant carrying delayed/enabled/index) if Key can carry one; otherwise cache the marker key per (index, enabled, delayed) in the sliver. Keep marker_spec's decoding in sync.

### A262-02 - crates/incular-widgets/src/scrolling/reorderable.rs:83 (§2, medium)
Evidence:
```
let mut marked = std::mem::replace(widget, empty_widget());
let replacement = empty_widget().kind().clone();
```
Suggested fix: Build the replacement kind directly instead of allocating a throwaway Widget (Rc) just to clone its kind:
let kind = std::mem::replace(
    marked.kind_mut(),
    WidgetKind::Box {
        size: incular_core::Size::ZERO,
        color: incular_core::Color::rgba(0, 0, 0, 0),
    },
);

### A262-01 - crates/incular-widgets/src/scrolling/reorderable.rs:194 (§2, high)
Evidence:
```
fn contains_marker(widget: &Widget) -> bool {
    let mut stack = vec![widget];
```
Suggested fix: Make the walk allocation-free by recursing over the child iterator:
fn contains_marker(widget: &Widget) -> bool {
    marker_spec(widget).is_some()
        || widget.kind().structure().children.into_iter().any(contains_marker)
}

### A270-03 - crates/incular-widgets/src/selection.rs:155 (§10, medium)
Evidence:
```
listeners: HashMap<u64, Rc<dyn Fn() + 'static>>,
next_geometry_listener: u64,
geometry_listeners: HashMap<u64, Rc<dyn Fn(SelectionGeometry) + 'static>>,
```
Suggested fix: Listener tokens are monotonically increasing u64s, but notification order comes from HashMap iteration (random per process via RandomState), so observers fire in an arbitrary order. Use `std::collections::BTreeMap` for `listeners` (line 155), `geometry_listeners` (line 157) and `SelectionNotifierState::listeners` (line 459): iteration becomes registration order, matching Flutter's ChangeNotifier, with no extra sort.

### A270-01 - crates/incular-widgets/src/selection.rs:348 (§2, medium)
Evidence:
```
state.listeners.values().cloned().collect::<Vec<_>>(),
state.geometry_listeners.values().cloned().collect::<Vec<_>>(),
```
Suggested fix: Every selection change collects two fresh Vecs of Rc callbacks (set_selection_snapshot, lines 348-353), and register() does the same again per notification at lines 568-573 (`let listeners = state.borrow().listeners.values().cloned().collect::<Vec<_>>();`). Keep a cached `Rc<[Rc<dyn Fn()>]>` (and the geometry equivalent) in the state, rebuilt only in add_listener/remove_listener (rare), and clone the Rc per notification instead of collecting a Vec. Same for the register() closure.

### A270-02 - crates/incular-widgets/src/selection.rs:354 (§2, medium)
Evidence:
```
state.geometry.clone(),
);
// geometry clone is taken even when there are no geometry listeners
```
Suggested fix: The tuple in set_selection_snapshot always clones the whole SelectionGeometry (including selection_rects Vec) on every change, even when `state.geometry_listeners` is empty. Compute the snapshot only when needed inside the block: `let geometry_snapshot = (!state.geometry_listeners.is_empty()).then(|| state.geometry.clone());` and only use it in the listener loop at line 365 (`if let Some(geometry) = geometry_snapshot { ... }`).

### A277-02 - crates/incular-widgets/src/tree/descriptors.rs:881 (§2, low)
Evidence:
```
let mut slots = Vec::new();
let mut widgets = Vec::new();
```
Suggested fix: Capture the bound before `children` is consumed and pre-size both vectors: `let capacity = children.len().min(placeholders.len());` then `let mut slots = Vec::with_capacity(capacity);` and `let mut widgets = Vec::with_capacity(capacity);`.

### A277-01 - crates/incular-widgets/src/tree/descriptors.rs:885 (§3, low)
Evidence:
```
if let Some(slot) = (0..placeholders.len())
    .find(|&slot| placeholders[slot].id == id && !slots.contains(&slot))
```
Suggested fix: if let Some(slot) = placeholders
    .iter()
    .enumerate()
    .find_map(|(slot, span)| (span.id == id && !slots.contains(&slot)).then_some(slot))

### A278-01 - crates/incular-widgets/src/tree/descriptors.rs:1593 (§11, medium)
Evidence:
```
let field = move |context_menu| {
    let mut spec = spec.clone();
    spec.context_menu = context_menu;
```
Suggested fix: Move the spec into the field builder and clone only on the context-menu path, so the None branch (the common case) performs no TextFieldSpec clone:

let field = move |spec: TextFieldSpec| {
    Widget::from_kind(WidgetKind::TextField(Box::new(spec)))
        .with_text_input_hints(input_type, input_action)
};
match context_menu_builder {
    Some(builder) => crate::context_menu::with_text_context_menu(
        move |state| {
            let mut spec = spec.clone();
            spec.context_menu = Some(state);
            field(spec)
        },
        builder,
    ),
    None => field(spec),
}

### A289-03 - crates/incular-widgets/src/tree/layout/behavior/text.rs:33 (§2, medium)
Evidence:
```
.map(|inline| inline.placeholders.to_vec())
.unwrap_or_default();
if let Some(inline) = inline {
```
Suggested fix: Avoid a fresh Vec per inline-text layout: keep a reusable placeholder scratch buffer on WidgetTree, `let mut placeholders = std::mem::take(&mut self.placeholder_scratch); placeholders.clear(); if let Some(inline) = inline { placeholders.extend_from_slice(&inline.placeholders); }`, and move it back into `self.placeholder_scratch` after the layout is produced.

### A289-04 - crates/incular-widgets/src/tree/layout/behavior/text.rs:108 (§3, medium)
Evidence:
```
.collect::<Vec<_>>()
.into(),
None => ChildOffsets::default(),
```
Suggested fix: ChildOffsets already implements FromIterator<Offset>, which keeps one or two offsets inline. Remove the intermediate Vec: delete `.collect::<Vec<_>>()` and `.into()` so the chain ends in `.collect()` directly.

### A289-05 - crates/incular-widgets/src/tree/layout/behavior/text.rs:375 (§2, high)
Evidence:
```
let whole = incular_text::StyledTextRun {
text: display.to_owned(),
style: style.clone(),
```
Suggested fix: Check the decoration before building the run, so the common undecorated field allocates nothing: `if !style.decoration.is_some_and(|decoration| !decoration.is_empty()) { return Arc::from([]); }` (or a shared empty Arc), then build `whole` only in the decorated case: `Arc::from([incular_text::StyledTextRun { text: display.to_owned(), style: style.clone(), range: 0..display.len() }])`.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
