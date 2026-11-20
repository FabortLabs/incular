# Fix brief incular-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `harness/incular/src/lib.rs`. Other agents are editing other files at
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
### A315-01 - harness/incular/src/lib.rs:453 (§2, low)
Evidence:
```
let names: Vec<String> = PROBES.with(|probes| probes.borrow().iter().cloned().collect());
let tree = self.runtime.as_ref().expect("mounted").tree();
let mut probes = serde_json::Map::new();
```
Suggested fix: Build the probe map inside the thread-local closure so the names are borrowed, not collected into a Vec<String> every frame: `let probes = PROBES.with(|names| { let mut probes = serde_json::Map::new(); for name in names.borrow().iter() { ...same body... probes.insert(name.clone(), json!({...})); } probes });`. The name clone is then only the owned map key. No other borrow conflicts: PROBES and self.runtime are separate.

### A315-02 - harness/incular/src/lib.rs:701 (§2, low)
Evidence:
```
let trace: Value =
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
trace["frames"]
    .as_array()
    .cloned()
```
Suggested fix: Take the frames array instead of cloning the whole array: `let mut trace: Value = ...; match trace["frames"].take() { Value::Array(frames) => Ok(frames), _ => Err(format!("{}: no frames", path.display())) }`. Make `trace` mutable. Same result, no copy of every frame.

### A315-04 - harness/incular/src/lib.rs:718 (§2, low)
Evidence:
```
let (Some(a), Some(b)) = (numbers(flutter), numbers(incular)) else {
    return if flutter == incular {
```
Suggested fix: Avoid two Vec allocations per field per frame for the common scalar case: match `(flutter, incular)` against `(Value::Number(x), Value::Number(y))` first and compare `x.as_f64()`/`y.as_f64()` directly, falling back to the `numbers()` Vec path only for arrays. Keep the same tolerance logic (scaled/raw) so results are unchanged.

### A315-03 - harness/incular/src/lib.rs:796 (§3, low)
Evidence:
```
for index in 0..frames {
    let (a, b) = (&theirs[index], &ours[index]);
    let t_us = a["t_us"].as_u64().unwrap_or_default();
```
Suggested fix: Iterate the zipped pair instead of indexing both vectors by a computed bound: `for (index, (a, b)) in theirs.iter().zip(&ours).enumerate() {` and drop the `let (a, b)` line. zip already stops at the shorter length, so `frames` (the min) is no longer needed in the loop; the `result.frames = frames;` assignment above it stays.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
