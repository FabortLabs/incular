# Fix brief incular-web-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-web/src/semantics.rs`, `crates/incular-web/src/clipboard.rs`, `crates/incular-web/src/fonts.rs`, `crates/incular-web/src/text_input.rs`. Other agents are editing other files at
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
### A166-03 - crates/incular-web/src/semantics.rs:204 (§10, medium)
Evidence:
```
for index in (0..current.length()).rev() {
if let Some(element) = current.item(index)
&& !listed
```
Suggested fix: Replace the membership scan over `listed` (a Vec of JS elements compared with `==`, one JS call per pair, O(children x listed) per changed parent) with a set of ids: build `let listed_ids: HashSet<u64> = node.children.iter().copied().collect();` once, and test each current child by its OWNER_ATTRIBUTE id (`element.get_attribute(OWNER_ATTRIBUTE).and_then(|id| id.parse().ok())`) against `listed_ids`. Keep `listed` only for the append loop below.

### A166-05 - crates/incular-web/src/semantics.rs:279 (§2, low)
Evidence:
```
.filter(|existing| existing.element.tag_name().eq_ignore_ascii_case(tag))
```
Suggested fix: `tag_name()` converts a JS string into a new Rust String for every existing node on every publish. Store the tag (`tag: &'static str`, as returned by element_kind) in NodeElement and compare `existing.tag == tag` instead.

### A166-02 - crates/incular-web/src/semantics.rs:298 (§23, medium)
Evidence:
```
let _ = element.set_attribute(
    "style",
    &format!(
```
Suggested fix: Skip unchanged work: add `bounds: Rect` (or the four f64 values) to NodeElement, and only rebuild the style string and call set_attribute when the node is new or its bounds differ from the stored value. This removes a format! allocation and a DOM style write per node on every publish, for nodes that did not move.

### A166-01 - crates/incular-web/src/semantics.rs:312 (§2, medium)
Evidence:
```
semantic.label.clone()
} else if is_text_input(semantic.role) {
semantic.value.clone()
```
Suggested fix: Borrow instead of cloning: `let text = if leaf_text { semantic.label.as_deref() } else if is_text_input(semantic.role) { semantic.value.as_deref() } else { None };` and at line 321 replace `let next = text.clone().unwrap_or_default();` with `let next = text.unwrap_or_default();` (the `text.clone()` is redundant; `text` is not used afterwards). `set_text_content(Some(next))` and `current != next` accept &str unchanged.

### A166-04 - crates/incular-web/src/semantics.rs:455 (§20, medium)
Evidence:
```
on_click.forget();
...
on_focus.forget();
...
forward.forget();
```
Suggested fix: Each created node element calls `forget()` on four closures (click, focus, keydown, keyup; lines 455, 463, 469), so every node created during the session leaks its closures for the page lifetime even after the element is removed. Store the closures in NodeElement (e.g. `listeners: Vec<Closure<dyn FnMut(web_sys::Event)>>`, keyboard ones as their own type) and, when a node is removed (the `update.removed` loop and the replace path), remove the listeners from the element first and then drop the closures. Keep the element-owned closures alive for as long as the element is attached.

### A164-01 - crates/incular-web/src/clipboard.rs:73 (§2, medium)
Evidence:
```
let buffer = BUFFER
    .with_borrow(Clone::clone)
    .ok_or(ClipboardError::Unavailable)?;
```
Suggested fix: let filtered = BUFFER
    .with_borrow(|buffer| buffer.as_ref().map(|buffer| buffer.filtered(&request)))
    .ok_or(ClipboardError::Unavailable)?;
if filtered.is_empty() {
    Err(ClipboardError::Unavailable)
} else {
    Ok(filtered)
}

### A164-02 - crates/incular-web/src/clipboard.rs:135 (§2, medium)
Evidence:
```
.chain(self.html.clone().map(TransferRepresentation::html))
.chain(self.image.clone().map(TransferRepresentation::rgba8_image))
```
Suggested fix: Make transfer() consume Contents (fn into_transfer(self) -> Option<DataTransfer>) so the pixel buffer and strings are moved, not cloned. In write(), call contents.write_system() before contents.into_transfer() (write_system is a no-op when there is no content). Make store() take self too, so take_paste() moves the Contents instead of cloning the RGBA image.

### A164-03 - crates/incular-web/src/fonts.rs:55 (§18, low)
Evidence:
```
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
cdn_path: &'static str,
```
Suggested fix: #[cfg_attr(not(target_arch = "wasm32"), expect(dead_code, reason = "only the wasm32 page module fetches fonts from the CDN"))]

### A164-04 - crates/incular-web/src/fonts.rs:276 (§2, low)
Evidence:
```
let language = language.to_ascii_lowercase();
if language.starts_with("ja") {
```
Suggested fix: Compare without allocating: add `fn has_prefix_ignore_ascii_case(tag: &str, prefix: &str) -> bool { tag.get(..prefix.len()).is_some_and(|head| head.eq_ignore_ascii_case(prefix)) }` and use it for each prefix check in cjk_for_language, removing the to_ascii_lowercase() String.

### A167-01 - crates/incular-web/src/text_input.rs:197 (§6, low)
Evidence:
```
    let autocomplete = match &traits.autofill_hints {
        Some(hints) => hints
            .first()
```
Suggested fix: let autocomplete = traits.autofill_hints.as_ref().map_or("off", |hints| {
    hints.first().map_or("on", |hint| browser_autofill_hint(hint))
});


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
