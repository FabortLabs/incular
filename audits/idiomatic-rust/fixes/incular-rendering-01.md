# Fix brief incular-rendering-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-rendering/src/compositor.rs`, `crates/incular-rendering/src/display_list.rs`, `crates/incular-rendering/src/rsuperellipse.rs`. Other agents are editing other files at
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
### A118-03 - crates/incular-rendering/src/compositor.rs:1441 (§2, medium)
Evidence:
```
let mut markers = self
 .partitioned
 .iter()
 .filter_map(|id| Some((id.raw(), self.layers.get(id.0)?.surface_partition?)))
 .collect::<Vec<_>>();
```
Suggested fix: Reuse a retained buffer instead of collecting a fresh Vec each time partitions change: keep a scratch `Vec<(u64, SurfacePartition)>` field (or reuse `self.flattened_partitions` by building into a scratch Vec and swapping it in when it differs). E.g. clear a scratch vec, `extend(...)`, `sort_unstable_by_key`, then `if scratch != self.flattened_partitions { std::mem::swap(&mut scratch, &mut self.flattened_partitions); self.scene_dirty = true; }`. Note the comparison and the swap must keep the stored order sorted as before.

### A118-02 - crates/incular-rendering/src/compositor.rs:1770 (§2, high)
Evidence:
```
.map(|layer| layer.children.clone())
 .unwrap_or_default()
 .into_iter()
```
Suggested fix: Iterate the child indices without copying the list, since subtree_bounds takes &self:
let source_bounds = (0..self.layers.get(id.0).map_or(0, |layer| layer.children.len()))
    .filter_map(|index| {
        let child = *self.layers.get(id.0)?.children.get(index)?;
        self.subtree_bounds(child, world_transform)
    })
    .reduce(union_rect);

### A119-04 - crates/incular-rendering/src/compositor.rs:1863 (§23, medium)
Evidence:
```
generation: self.subtree_generation(id),
bounds,
});
```
Suggested fix: subtree_generation and subtree_bounds each walk the full subtree of an effect layer, and flatten_layer calls them again for every nested effect, so a nested effect's subtree is re-walked once per enclosing effect per frame. Compute both values in one bottom-up pass over the subtree (or cache `(subtree_generation, subtree_bounds)` on the layer keyed by its `generation` and invalidated when a descendant changes) and reuse them in flatten_layer and write_debug_at.

### A119-05 - crates/incular-rendering/src/compositor.rs:2653 (§5, low)
Evidence:
```
out.push_str(&format!("{indent}{id:?} parent={parent:?} {kind}\n"));
```
Suggested fix: Use `writeln!(out, "{indent}{id:?} parent={parent:?} {kind}")` (with `use std::fmt::Write as _;`), which removes the intermediate String. Only the debug_tree dump path is affected, so this is a readability/cost nit, not per frame.

### A120-07 - crates/incular-rendering/src/display_list.rs:445 (§6, medium)
Evidence:
```
let end = index.min(commands.len() - 1);
...
entry.extend_from(&Self::lift_partitions_in(&commands[start + 1..end]));
```
Suggested fix: Panics when the last command is an unmatched `PushSurfacePartition`: `index` reaches `len`, so `end == start == len - 1` and `commands[start + 1..end]` is an inverted range. Track whether the partition closed: `let closed = index < commands.len(); let inner_end = index.min(commands.len());` then slice `&commands[start + 1..inner_end]`, and append `PopSurfacePartition` only when `closed`. (Marker pairing is a framework invariant, so this is a latent panic, not an app-author error.)

### A120-02 - crates/incular-rendering/src/display_list.rs:475 (§2, medium)
Evidence:
```
for entry in &lifted {
    base.extend_from(entry);
}
```
Suggested fix: The lifted entries are owned and only appended, so move them instead of cloning every command: `base.commands_mut().append(lifted.commands_mut());` (with `lifted` as one `Self`/`Vec<PaintCommand>` accumulator, see the preceding finding). Clones each lifted command a second time per frame otherwise.

### A124-03 - crates/incular-rendering/src/rsuperellipse.rs:40 (§18, low)
Evidence:
```
#[allow(clippy::cast_possible_truncation)]
fn offset(self) -> Offset {
    Offset::new(self.x as f32, self.y as f32)
```
Suggested fix: Replace #[allow] with #[expect] and a reason. Same change for the #[allow(...)] blocks at lines 235 and 276 (superellipse_bezier_factors and compute_n_and_xj):
#[expect(clippy::cast_possible_truncation, reason = "path geometry is f32 while the Flutter superellipse math is f64")]


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
