# Fix brief incular-controls-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-controls/src/composite.rs`, `crates/incular-controls/src/checkbox.rs`, `crates/incular-controls/src/progress.rs`, `crates/incular-controls/src/slider.rs`, `crates/incular-controls/src/text_input.rs`, `crates/incular-controls/src/toast.rs`, `crates/incular-controls/src/scrollbar.rs`. Other agents are editing other files at
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
### A014-02 - crates/incular-controls/src/composite.rs:85 (§2, low)
Evidence:
```
items: Rc<Vec<CompositeItem>>,
```
Suggested fix: Store the immutable shared list as `items: Rc<[CompositeItem]>`; in `set_items` use `self.items = items.into_iter().collect();` and in `items()` return `&self.items` (drops one indirection and the Vec-wrapper; collect into Rc<[T]> directly where the iterator is exact-size).

### A014-01 - crates/incular-controls/src/composite.rs:164 (§6, low)
Evidence:
```
let origin = start.expect("checked above");
```
Suggested fix: Replace the `if start.is_none() { return ... }` block and the `expect` with a single let-else: `let Some(origin) = start else { return if direction >= 0 { self.first() } else { self.last() }; };`

### A014-05 - crates/incular-controls/src/checkbox.rs:260 (§2, low)
Evidence:
```
Widget::environment_scope(theme.clone(), checkbox.into())
```
Suggested fix: `build_with_group` takes `theme: ControlTheme` by value (it is private). Public `build(&self, theme: &ControlTheme)` calls `self.build_with_group(theme.clone(), None)`; `From<Root>` passes its owned `theme` directly instead of `&theme`, removing a full ControlTheme clone (a large struct) per checkbox build.

### A017-01 - crates/incular-controls/src/progress.rs:61 (§8, medium)
Evidence:
```
self.value = Some(f32::NAN);
self.value.is_some_and(f32::is_nan)
self.value.filter(|value| value.is_finite())
```
Suggested fix: Replace the NaN sentinel inside Option<f32> with an enum and keep the public builder API unchanged:
#[derive(Clone, Copy, Debug, PartialEq)]
enum ProgressValue { Determinate(f32), Indeterminate }
Field: `value: ProgressValue` with `#[default(ProgressValue::Determinate(0.))]` and `#[builder(default = ProgressValue::Determinate(0.))]`.
value(v) sets Determinate(v); indeterminate(true) sets Indeterminate; indeterminate(false) sets Determinate(self.min) only when currently Indeterminate.
is_indeterminate() becomes matches!(self.value, ProgressValue::Indeterminate); normalized_value() matches Determinate(v) and returns None for Indeterminate; the semantics numeric_value becomes match self.value { Determinate(v) if v.is_finite() => Some(f64::from(v)), _ => None }.

### A019-01 - crates/incular-controls/src/slider.rs:841 (§1, low)
Evidence:
```
callback: &Option<Rc<dyn Fn(f32) + 'static>>,
) {
    if (next - value.get()).abs() <= f32::EPSILON {
```
Suggested fix: Take the borrowed form: `callback: Option<&dyn Fn(f32)>` in `fn update_value`, keep the body (`if let Some(callback) = callback { callback(next); }`), and change the three call sites from `&callback` (line 620), `&increment_callback` (line 817) and `&decrement_callback` (line 829) to `callback.as_deref()` / `increment_callback.as_deref()` / `decrement_callback.as_deref()`.

### A021-01 - crates/incular-controls/src/text_input.rs:64 (§8, low)
Evidence:
```
multiline: bool,
    paragraph_layout: bool,
```
Suggested fix: Replace both private bool fields with one layout enum: `#[derive(Clone, Copy, Default, PartialEq, Eq)] enum EditorLayout { #[default] SingleLine, Multiline, Paragraphs }` and a field `#[builder(default, setter(skip))] layout: EditorLayout`. Use `layout != EditorLayout::SingleLine` where `multiline` is read, and `layout == EditorLayout::Paragraphs` where `paragraph_layout` is read. TextArea sets `Multiline`, DocumentEditor sets `Paragraphs`, so the single-line-with-paragraph-layout combination becomes unrepresentable.

### A022-01 - crates/incular-controls/src/toast.rs:224 (§2, low)
Evidence:
```
let entry = ToastEntry {
    id: ToastId(0),
    title: value.title.clone(),
```
Suggested fix: Stop building a placeholder ToastEntry. Change `fn from_entry(entry: &ToastEntry, theme: &ControlTheme)` to `fn from_parts(title: &str, description: Option<&str>, theme: &ControlTheme)` (use `title` and `description` in place of `entry.title` and `entry.description`). In `Provider::build` call `Root::from_parts(&entry.title, entry.description.as_deref(), theme)`. In `From<Root>` delete the `let entry = ...` block and write `let visual = value.child.clone().unwrap_or_else(|| Root::from_parts(&value.title, value.description.as_deref(), &theme));`. This removes the dummy `ToastId(0)` and the title/description clones made even when `child` is set.

### A329-01 - crates/incular-controls/src/scrollbar.rs:88 (§6, low)
Evidence:
```
Some((_, child)) => (controller.unwrap(), child),
```
Suggested fix: Some((_, child)) => (controller.unwrap_or_default(), child),  // controller is always Some here: the early return above handles the None + existing-viewport case, so this is behavior-identical and removes the unexplained unwrap


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
