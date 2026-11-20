# Fix brief examples-03

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `examples/workbench/main.rs`, `examples/canvas_layers/main.rs`, `examples/devtools_playground/text_lab.rs`, `examples/devtools_playground/main.rs`, `examples/material_gallery/main.rs`, `examples/material_workbench/main.rs`, `examples/snake/view.rs`, `examples/studio/commands.rs`, `examples/studio/restoration.rs`, `examples/studio/theme.rs`, `examples/studio/views/components.rs`. Other agents are editing other files at
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
### A334-02 - examples/workbench/main.rs:75 (§2, low)
Evidence:
```
match (locale.to_string().as_str(), key) {
(value, "greeting") if value.starts_with("fr") => Some("Bonjour"),
(_, "greeting") => Some("Hello"),
```
Suggested fix: Avoid allocating a String per message lookup: match on the language subtag borrowed from the Locale (e.g. the ICU4X language accessor, if the locale type exposes one) instead of `locale.to_string()`. Keep as is if no borrowed accessor exists.

### A334-01 - examples/workbench/main.rs:203 (§7, medium)
Evidence:
```
let route = app_navigator.current().expect("home route");
// Pop route action (lines 365-372) calls nav.pop() with no minimum depth check; maybe_pop() pops the root route too.
```
Suggested fix: Stop the Pop action from removing the last route so the invariant holds: in the 'Pop route' closure use `if nav.routes().len() > 1 { let _ = nav.pop(); }` before the revision bump. Alternatively, render an empty placeholder when `app_navigator.current()` is None instead of expect().

### A316-01 - examples/canvas_layers/main.rs:14 (§2, low)
Evidence:
```
    let mut pixels = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
```
Suggested fix:     let mut pixels = Vec::with_capacity(8 * 8 * 4);

### A318-02 - examples/devtools_playground/text_lab.rs:62 (§3, low)
Evidence:
```
        Column::new(
            [11., 14., 18., 24., 32., 44.]
                .into_iter()
```
Suggested fix: Drop the `.collect::<Vec<Widget>>()` at the end of the chain and pass the map iterator directly: `Column::new([11., 14., 18., 24., 32., 44.].into_iter().map(|size| text(...)))`. Column::new accepts any IntoIterator of Into<Widget>.

### A318-04 - examples/devtools_playground/main.rs:175 (§2, low)
Evidence:
```
let mut items = vec![text("SECTIONS", 11., FontWeight::W700, MUTED), gap(8.)];
for section in Section::ALL {
    items.push(item);
```
Suggested fix: Length is known up front (2 fixed entries plus two pushes per section): `let mut items = Vec::with_capacity(2 + 2 * Section::ALL.len());` before the `vec![...]` is replaced by `let mut items = Vec::with_capacity(...); items.push(...); items.push(...);`.

### A321-01 - examples/material_gallery/main.rs:34 (§3, low)
Evidence:
```
std::iter::once(Text::new(title).into())
.chain(children)
.collect::<Vec<_>>(),
```
Suggested fix: Drop the intermediate Vec and pass the iterator straight to Column::new, which accepts impl IntoIterator<Item = impl Into<Widget>>:
    Column::new(
        std::iter::once(Text::new(title).into()).chain(children),
    )

### A321-02 - examples/material_workbench/main.rs:19 (§3, low)
Evidence:
```
Column::new(
std::iter::once(Text::new(title).into())
.chain(children)
```
Suggested fix: Drop the intermediate Vec: Column::new(std::iter::once(Text::new(title).into()).chain(children)) with no .collect::<Vec<_>>(), since Column::new takes impl IntoIterator<Item = impl Into<Widget>>.

### A325-01 - examples/snake/view.rs:17 (§1, medium)
Evidence:
```
pub fn root(game: Game, viewport: Size) -> Widget {
```
Suggested fix: Take the snapshot by reference: `pub fn root(game: &Game, viewport: Size) -> Widget` (body already only reads it via `board(&game, cell)`, change to `board(game, cell)`). Caller in examples/snake/main.rs:65 becomes `view::root(&snapshot, cx.viewport())`, so the owned `Game` (and its `snake` Vec) is no longer moved in per build.

### A325-02 - examples/studio/commands.rs:5 (§18, low)
Evidence:
```
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
```
Suggested fix: Replace with `#[expect(dead_code, reason = "...")]` (same for the `#[allow(dead_code)]` on `matches_key` at line 77). Only do this if the lint actually fires in the example build, because an unfulfilled `#[expect]` is a warning that -D warnings turns into an error.

### A325-03 - examples/studio/restoration.rs:17 (§8, medium)
Evidence:
```
pub theme_mode: String,
pub locale_code: String,
```
Suggested fix: Store the typed values and let serde handle the file format: `pub theme_mode: ThemeMode,` and `pub locale: StudioLocale,` (derive Serialize/Deserialize on them, with `#[serde(rename)]` if the on-disk strings must stay `Dark` / `en-US`). This removes the `format!("{:?}")` and `match restoration.theme_mode.as_str()` round trip in examples/studio/state.rs:157-162 and :265-266. Default at line 36-37 becomes `ThemeMode::Dark` / `StudioLocale::English`.

### A326-04 - examples/studio/theme.rs:13 (§18, low)
Evidence:
```
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct StudioTheme {
```
Suggested fix: Replace with `#[expect(dead_code, reason = "palette fields are kept for every theme slot even when a view does not read them")]`. Confirm the expectation is fulfilled (some field is unread); if none is, remove the attribute instead.

### A326-05 - examples/studio/views/components.rs:3 (§18, low)
Evidence:
```
#![allow(dead_code)]

use super::super::theme::StudioTheme;
```
Suggested fix: Replace with `#![expect(dead_code, reason = "shared component helpers are used by a subset of views")]`. Confirm the expectation is fulfilled by the build; drop the attribute if no helper is unused.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
