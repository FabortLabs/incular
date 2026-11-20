# Fix brief incular-widgets-10

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/forms.rs`. Other agents are editing other files at
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
### A224-02 - crates/incular-widgets/src/forms.rs:91 (§10, low)
Evidence:
```
listeners: HashMap<usize, Rc<dyn Fn()>>,
```
Suggested fix: Use `listeners: BTreeMap<usize, Rc<dyn Fn()>>` (add BTreeMap to the collections import; HashMap stays for `fields`). Listener tokens are monotonically increasing, so iteration becomes deterministic registration order instead of per-process random HashMap order. Pairs with the snapshot fix at line 524.

### A224-03 - crates/incular-widgets/src/forms.rs:269 (§2, medium)
Evidence:
```
let all_error_free = fields
.all(|(_, field)| field.error_entry().is_none());
```
Suggested fix: Add `fn has_error_entry(&self) -> bool;` to the private FormEntry trait, implement it as `self.error.borrow().is_some()` in FieldState and GenericFieldState, and write `let all_error_free = fields.iter().all(|(_, field)| !field.has_error_entry());`. Today each field's error String is cloned only to test for None.

### A224-01 - crates/incular-widgets/src/forms.rs:524 (§11, high)
Evidence:
```
state.listeners.values().cloned().collect::<Vec<_>>()
```
Suggested fix: Keep a snapshot in FormRegistry: add `listener_snapshot: Rc<[Rc<dyn Fn()>]>`, rebuild it only in add_listener and remove_listener (rare), and in notify_form_listeners take `let listeners = Rc::clone(&state.listener_snapshot);` (inside the existing borrow_mut block) then `for listener in listeners.iter() { listener(); }`. Every field edit calls this (via the controller observer at line 172), so the per-notification Vec goes away. Snapshot semantics (listeners removed mid-pass still run once) are unchanged.

### A224-06 - crates/incular-widgets/src/forms.rs:802 (§2, medium)
Evidence:
```
match &self.options_builder {
Some(builder) => builder(value),
None => self.options.clone(),
```
Suggested fix: Store the fixed options as `Rc<[T]>` (in RawAutocomplete and RawAutocompleteState) and have build_options return `Rc<[T]>`: `None => Rc::clone(&self.options)`, `Some(builder) => builder(value).into()`. Today every text change deep-clones the whole fixed options Vec<T> even though it never changes. Pairs with the layout clone finding at line 1037.

### A224-05 - crates/incular-widgets/src/forms.rs:875 (§11, medium)
Evidence:
```
let value = controller.value();
let focused = focus.has_focus();
let text_changed = self.last_text.borrow().as_deref() != Some(value.text.as_str());
```
Suggested fix: Compare without cloning the text on every build: `let text_changed = controller.with_value(|value| self.last_text.borrow().as_deref() != Some(value.text.as_str()));`. Then fetch the owned value only when text changed: `if text_changed && !self.selecting.get() { let value = controller.value(); ... }` (and similarly `else if text_changed { let value = controller.value(); ... }`). controller.value() (editing.rs:401) clones the String; with_value (editing.rs:409) does not. Do not call build_options inside with_value, since the callback may reenter the controller.

### A225-02 - crates/incular-widgets/src/forms.rs:875 (§2, medium)
Evidence:
```
let value = controller.value();
let focused = focus.has_focus();
let text_changed = self.last_text.borrow().as_deref() != Some(value.text.as_str());
```
Suggested fix: `TextEditingController::value()` (incular-text/src/editing.rs:401) clones the whole TextEditingValue, including its String, on every call. `sync` runs on every build of the autocomplete and clones only to compare text. Add a borrowing accessor such as `pub fn with_value<R>(&self, f: impl FnOnce(&TextEditingValue) -> R) -> R` (calling `self.changes.track()` first) and run the text comparison inside it, cloning `value.text` only when text_changed is true. The clone at lines 879 and 887 is already conditional.

### A224-07 - crates/incular-widgets/src/forms.rs:1037 (§11, high)
Evidence:
```
let options = state.options.borrow().clone();
```
Suggested fix: Change RawAutocompleteState.options from `RefCell<Vec<T>>` to `RefCell<Rc<[T]>>` (sync at line 880 stores `Rc::from(...)`, or the build_options change above). Then line 1037 becomes `let options = Rc::clone(&state.options.borrow());`. The overlay_child_layout_builder closure at lines 1057-1058 runs during overlay layout and currently does `(view.clone(), select.clone(), options.clone())`, which deep-clones the whole Vec<T> on every layout pass while the options are showing; with Rc<[T]> that is a refcount bump. Also consider hoisting the `Rc::default()` passed to Widget::stateful_builder at line 1061 into a field of the captured state if that API allows, since it allocates per layout pass.

### A225-01 - crates/incular-widgets/src/forms.rs:1037 (§2, high)
Evidence:
```
let options = state.options.borrow().clone();
let highlighted = state.highlighted.get();
// line 1058: let (view, select, options) = (view.clone(), select.clone(), options.clone());
```
Suggested fix: Store the options as a shared slice so per-build and per-layout copies are reference-count bumps. Change line 813 to `options: RefCell<Rc<[T]>>`, init at line 962 with `options: RefCell::new(Rc::from([]))`, and in `sync` (line 880) assign `*self.options.borrow_mut() = config.build_options(&value).into();`. Line 1037 then becomes `let options = state.options.borrow().clone();` (an Rc clone), and line 1058 keeps `options.clone()` as a cheap Rc clone. The call `view(context, select.clone(), &options)` at line 1062 still coerces &Rc<[T]> to &[T]. Only text changes allocate, as before.

### A224-08 - crates/incular-widgets/src/forms.rs:1463 (§23, medium)
Evidence:
```
let end = icu_segmenter::GraphemeClusterSegmenter::new()
.segment_str(&value.text)
```
Suggested fix: Skip segmentation when it cannot matter: `if enforce && value.text.len() > self.max_length { let end = ...; value.text.truncate(end); }`. A grapheme cluster is at least one byte, so text no longer than max_length bytes holds at most max_length clusters and the `nth(self.max_length)` lookup would find nothing and truncate to no change. The current code segments the whole text on every edit (keystroke) even for short text.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
