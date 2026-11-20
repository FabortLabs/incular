# Fix brief incular-devtools-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `tools/incular-devtools/src/views/property_editor.rs`. Other agents are editing other files at
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
### A051-02 - tools/incular-devtools/src/views/property_editor.rs:70 (§2, low)
Evidence:
```
let field = fields.entry(slot.to_owned()).or_insert_with(|| {
```
Suggested fix: Avoid the String allocation on every build for an existing slot: `if !fields.contains_key(slot) { fields.insert(slot.to_owned(), EditorField { controller: TextEditingController::with_text(text.to_owned()), synced: text.to_owned() }); }` then `let field = fields.get_mut(slot).expect("inserted above");`.

### A051-01 - tools/incular-devtools/src/views/property_editor.rs:406 (§19, medium)
Evidence:
```
expanded(editor.clone()),
```
Suggested fix: Decide the placement once so the widget is moved, not cloned: `let (head, detached) = if stacked { (row_of(vec![marker, expanded(label), revert]), Some(editor)) } else { (Row::new(vec![marker, label, expanded(editor), gap(4., 1.), revert]).cross_axis_alignment(CrossAxisAlignment::Start).into(), None) };` then `let mut rows = vec![head];` and `if let Some(editor) = detached { rows.push(Padding::new(EdgeInsets::from_ltrb(MARKER_SLOT, 0., REVERT_SLOT, 0.), editor).into()); }`.

### A051-03 - tools/incular-devtools/src/views/property_editor.rs:493 (§2, low)
Evidence:
```
let key = edit.key.clone();
```
Suggested fix: Borrow instead of cloning; `key` is only read by `format!("{key}#v")` in the Text and Raw arms, and `edit` is already a borrow: `let key = &edit.key;`.

### A051-04 - tools/incular-devtools/src/views/property_editor.rs:520 (§2, low)
Evidence:
```
DebugValue::Str(text) => text.clone(),
```
Suggested fix: `text` is only read (passed as `&text` to `env.editors.field`), so return a borrow: `let text: Cow<str> = match &property.value { DebugValue::Str(text) => Cow::Borrowed(text.as_str()), other => Cow::Owned(debug_value(other)) };` (add `use std::borrow::Cow;`).

### A051-05 - tools/incular-devtools/src/views/property_editor.rs:553 (§2, low)
Evidence:
```
DebugValue::Enum(text) => text.clone(),
```
Suggested fix: Make `enum_text` return `Cow<'_, str>` (`DebugValue::Enum(text) => Cow::Borrowed(text.as_str()), other => Cow::Owned(debug_value(other))`). All three callers (lines 504, 508, 512) only compare or borrow the result, and `&current` / `&enum_text(..)` deref-coerce to `&str`.

### A051-06 - tools/incular-devtools/src/views/property_editor.rs:810 (§2, low)
Evidence:
```
let mut swatches = Vec::new();
```
Suggested fix: `PALETTE` is a `[&str; 12]` const, so the length is known: `let mut swatches = Vec::with_capacity(PALETTE.len());`.

### A052-02 - tools/incular-devtools/src/views/property_editor.rs:810 (§2, low)
Evidence:
```
    let mut swatches = Vec::new();
    for hex in PALETTE {
```
Suggested fix: Length is known (PALETTE is `[&str; 12]`): `let mut swatches = Vec::with_capacity(PALETTE.len());`. The same pattern repeats with known or bounded lengths at lines 839 (`children`, options.len() * 2), 933 (`chips`, options.len()), 958 (`rows`, 3) and 960 (`cells`, 3); give them `with_capacity` too.

### A051-07 - tools/incular-devtools/src/views/property_editor.rs:839 (§2, low)
Evidence:
```
let mut children = Vec::new();
```
Suggested fix: The segment count is `options.len()` with a gap between each, so use `let mut children = Vec::with_capacity(options.len() * 2);`. The same applies to `let mut chips = Vec::new();` in `menu` (line 933), which holds one entry per option: `Vec::with_capacity(options.len())`.

### A052-01 - tools/incular-devtools/src/views/property_editor.rs:1059 (§2, low)
Evidence:
```
    let controllers: Vec<TextEditingController> = (0..4)
    ...
    let controllers = Rc::new(controllers);
```
Suggested fix: Collect straight into a shared slice and drop the Vec and the extra Rc::new: `let controllers: Rc<[TextEditingController]> = (0..4).map(|side| env.editors.field(&format!("{}#{side}", edit.key), &format_float(f64::from(current[side])))).collect();`. Existing uses (`controllers.iter()`, `controllers[side]`, `controllers.clone()`) work unchanged through Deref.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
