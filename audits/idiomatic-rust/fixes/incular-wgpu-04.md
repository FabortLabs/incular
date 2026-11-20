# Fix brief incular-wgpu-04

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-wgpu/src/glyph_uploads.rs`, `crates/incular-wgpu/src/glyphs.rs`, `crates/incular-wgpu/src/renderer/backdrop_cache.rs`. Other agents are editing other files at
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
### A170-02 - crates/incular-wgpu/src/glyph_uploads.rs:170 (§2, medium)
Evidence:
```
*self = Self::default();
// drops bitmaps, glyphs and runs capacity on every flush, which the design comment at 168 retains on purpose
```
Suggested fix: If steady-state reuse is preferred over releasing idle memory, replace the reset with `self.bitmaps.clear(); self.glyphs.clear(); self.runs.clear(); self.staged_bytes = 0;` (keeping capacity, optionally trimming above a threshold). This is a deliberate trade-off documented at lines 168-169, so only change it if the idle-retention cost is judged acceptable; otherwise leave as is.

### A171-01 - crates/incular-wgpu/src/glyphs.rs:878 (§2, medium)
Evidence:
```
let mut retired = Vec::new();
self.entries.retain(|key, entry| {
 retired.push(*key);
```
Suggested fix: Push straight into the persistent buffer instead of a temporary Vec, splitting the borrow of the two disjoint fields:

let retired_keys = &mut self.retired_keys;
self.entries.retain(|key, entry| {
    if usize::from(entry.page) == index {
        retired_keys.push(*key);
        false
    } else {
        true
    }
});

and delete the line `self.retired_keys.extend(retired);`.

### A176-04 - crates/incular-wgpu/src/renderer/backdrop_cache.rs:128 (§2, medium)
Evidence:
```
let needed: HashSet<_> = plans
    .iter()
    .zip(&capturers)
```
Suggested fix: Replace the per-frame HashSet with a scratch buffer on the renderer, or with a Vec<LayerId> filled and then sorted/deduped (plans are few), reused across frames via clear(). Membership is only queried with needed.contains(&layer) on a handful of layers.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
