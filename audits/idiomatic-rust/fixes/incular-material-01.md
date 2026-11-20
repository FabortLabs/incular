# Fix brief incular-material-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-material/src/catalog/pickers.rs`. Other agents are editing other files at
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
### A084-02 - crates/incular-material/src/catalog/pickers.rs:401 (AGENTS.md (delete unused code), low)
Evidence:
```
let _ = picker.initial_date;
```
Suggested fix: Delete this line and the initial_date field of CalendarDatePicker (line 275; its initialization is at line 305). The constructor parameter stays because it still seeds selected and displayed_month. The let _ = line only silences dead_code, which is a sign the field should not exist.

### A084-05 - crates/incular-material/src/catalog/pickers.rs:406 (§2, medium)
Evidence:
```
let text = theme.core().text_theme.clone();
(same clone at line 1235 in TimePickerDialog::from)
```
Suggested fix: theme is an owned local inside the builder closure and the text theme is only read there, so borrow it: let text = &theme.core().text_theme; Then drop the & at the call sites that pass &text (day_grid, year_grid, and dial at line 1360), since they already take a reference. Lines 406 and 1235 are the same pattern. Line 973 (DateRangePickerDialog) must keep its clone because the ListView::builder closure is 'static and moves text.

### A084-06 - crates/incular-material/src/catalog/pickers.rs:503 (§2, low)
Evidence:
```
let cell = match on_tap {
Some(on_tap) => GestureDetector::new(cell).on_tap(move || on_tap()).into(),
```
Suggested fix: day_cell takes Option<Rc<dyn Fn()>> and then wraps it in a second closure, but GestureDetector::on_tap already takes impl Fn() + 'static and boxes it itself. Make the parameter generic: on_tap: Option<impl Fn() + 'static>, pass it straight through with GestureDetector::new(cell).on_tap(on_tap).into(), and build the callers without the Rc, e.g. let on_tap = enabled.then(|| { let picker = picker.clone(); move || picker.select(date) }); (the same change applies to the enabled.then in range_month at line 1086-1089 with state.tap(date)). This removes one Rc and one closure allocation per enabled day on each rebuild.

### A084-03 - crates/incular-material/src/catalog/pickers.rs:597 (§2, medium)
Evidence:
```
let years = (first..=last).collect::<Vec<_>>();
let rows = years
.chunks(3)
```
Suggested fix: Collect nothing: the year range is already indexable. Replace lines 597-601 (and the .map(|chunk| chunk.to_vec()) copy) with: let row_count = usize::try_from(i64::from(last) - i64::from(first) + 1).unwrap_or(0).div_ceil(3); Pass row_count to ListView::builder, and in the row closure compute let start = first + (row * 3) as i32; then iterate (start..=last).take(3) in place of rows[row].iter().map(|&year| ...). This removes two Vecs (the year list and a Vec of Vec per chunk) on every rebuild. Empty ranges (first > last) still give zero rows.

### A084-04 - crates/incular-material/src/catalog/pickers.rs:642 (§3, low)
Evidence:
```
let mut cells = cells.collect::<Vec<_>>();
while cells.len() < 3 {
cells.push(Expanded::new(SizedBox::shrink()).into());
```
Suggested fix: Keep the iterator and pad it lazily instead of collecting and pushing (Row::new already takes any IntoIterator, see weekday_header). Replace lines 642-646 with: Row::new(cells.chain(std::iter::repeat_with(|| Widget::from(Expanded::new(SizedBox::shrink())))).take(3)) . The chunks(3) input guarantees at most three real cells, so take(3) always yields exactly three children.

### A085-01 - crates/incular-material/src/catalog/pickers.rs:1061 (§2, low)
Evidence:
```
    let mut cells: Vec<Widget> = (0..offset)
        .map(|_| Widget::from(Expanded::new(SizedBox::new().height(DAY_PICKER_ROW_HEIGHT))))
        .collect();
```
Suggested fix: let mut cells: Vec<Widget> = Vec::with_capacity(MAX_DAY_PICKER_ROW_COUNT * 7);
cells.extend((0..offset).map(|_| Widget::from(Expanded::new(SizedBox::new().height(DAY_PICKER_ROW_HEIGHT)))));
(The cell count is at most 6 leading blanks + 31 days, padded to a multiple of 7, which is at most MAX_DAY_PICKER_ROW_COUNT * 7 = 42, so the following pushes never regrow the Vec.)

### A085-02 - crates/incular-material/src/catalog/pickers.rs:1111 (§3, low)
Evidence:
```
    let mut week = Vec::new();
    for cell in cells {
        week.push(cell);
```
Suggested fix: Replace the manual push/mem::take loop with an iterator chunking loop that allocates each week once:
let mut cells = cells.into_iter();
loop {
    let week: Vec<Widget> = cells.by_ref().take(7).collect();
    if week.is_empty() {
        break;
    }
    rows.push(Row::new(week).into());
}
(Original: Vec::new() regrows 4 then 8 per week, and mem::take leaves a zero-capacity Vec for the next week.)

### A085-03 - crates/incular-material/src/catalog/pickers.rs:1402 (§2, low)
Evidence:
```
    let mut children: Vec<Widget> = vec![
        Container::new()
            .width(DIAL_SIZE)
```
Suggested fix: Start from a presized Vec, since 1 backdrop child plus 12 label Positioned children are pushed below:
let mut children: Vec<Widget> = Vec::with_capacity(13);
children.push(
    Container::new()
        .width(DIAL_SIZE)
        .height(DIAL_SIZE)
        .decoration(
            BoxDecoration::new()
                .shape(BoxShape::Circle)
                .color(scheme.surface_container_highest),
        )
        .into(),
);
(Original vec! of 1 element then 12 pushes regrows at 4, 8 and 16.)


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
