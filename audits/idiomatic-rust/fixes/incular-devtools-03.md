# Fix brief incular-devtools-03

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `tools/incular-devtools/src/views/command_palette.rs`, `tools/incular-devtools/src/diagnostics.rs`. Other agents are editing other files at
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
### A045-03 - tools/incular-devtools/src/views/command_palette.rs:388 (§3, low)
Evidence:
```
for (index, entry) in commands.iter_mut().take(ToolView::ALL.len()).enumerate() {
entry.shortcut = Some(NAV_KEYS[index]);
```
Suggested fix: for (entry, key) in commands.iter_mut().take(ToolView::ALL.len()).zip(NAV_KEYS) {
    entry.shortcut = Some(key);
}

### A045-01 - tools/incular-devtools/src/views/command_palette.rs:435 (§2, medium)
Evidence:
```
pub fn filter(commands: &[Command], query: &str) -> Vec<Command> {
if query.trim().is_empty() {
return commands.to_vec();
```
Suggested fix: Return borrowed entries so no Command (with its String title) is cloned: change the signature to `pub fn filter<'a>(commands: &'a [Command], query: &str) -> Vec<&'a Command>`, replace `return commands.to_vec();` with `return commands.iter().collect();`, and replace the final `.map(|(_, _, command)| command.clone()).collect()` with `.map(|(_, _, command)| command).collect()`. Callers in build() already work through `.iter()`, `.len()` and field access, which auto-deref &&Command.

### A045-02 - tools/incular-devtools/src/views/command_palette.rs:444 (§4, low)
Evidence:
```
ranked.sort_by_key(|(rank, index, _)| (*rank, *index));
```
Suggested fix: ranked.sort_unstable_by_key(|(rank, index, _)| (*rank, *index));  // keys are unique because of the catalogue index, so the result is identical

### A045-04 - tools/incular-devtools/src/views/command_palette.rs:513 (§2, low)
Evidence:
```
let mut offsets = Vec::new();
```
Suggested fix: let mut offsets = Vec::with_capacity(matches.len());  // exactly one offset is pushed per match

### A045-05 - tools/incular-devtools/src/views/command_palette.rs:773 (§3, low)
Evidence:
```
let mut groups = groups.into_iter();
let mut left = Vec::new();
for (index, group) in groups.by_ref().enumerate() {
```
Suggested fix: Drop the `let mut groups = groups.into_iter();` line and write `for (index, group) in groups.into_iter().enumerate() {` (the `by_ref()` is unnecessary because the iterator is not used afterwards).

### A037-05 - tools/incular-devtools/src/diagnostics.rs:404 (§10, low)
Evidence:
```
let mut seen = HashSet::new();
```
Suggested fix: Positions are dense indices into `self.nodes`, so replace the per-call `HashSet<usize>` (SipHash per row) with `let mut seen = vec![false; self.nodes.len()];` and `if !include(position) || std::mem::replace(&mut seen[position], true) { continue; }`.

### A037-01 - tools/incular-devtools/src/diagnostics.rs:406 (§2, medium)
Evidence:
```
.splice_hidden(self.roots.clone(), hide)
```
Suggested fix: Change `splice_hidden` to take `positions: &[usize]` (and build its pending stack from the slice in reverse) so the caller passes `&self.roots` instead of cloning the whole roots Vec on every `visible_with` call.

### A037-02 - tools/incular-devtools/src/diagnostics.rs:419 (§2, medium)
Evidence:
```
let children = self.splice_hidden(
    node.child_ids()
        .iter()
        .filter_map(|child| self.index.get(child).copied())
        .collect(),
```
Suggested fix: Avoid the per-visited-node `collect::<Vec<usize>>()` by passing children to `splice_hidden` without building a Vec: either iterate `node.child_ids()` lazily from a `&[usize]` of child positions, or have `splice_hidden` write into a `&mut Vec<usize>` scratch buffer that the walk clears and reuses across nodes.

### A037-03 - tools/incular-devtools/src/diagnostics.rs:448 (§2, low)
Evidence:
```
let mut pending = positions.into_iter().rev().collect::<Vec<_>>();
```
Suggested fix: If `positions` stays owned, reuse it: `let mut pending = positions; pending.reverse();`. Otherwise fold this into the slice/scratch-buffer change for `splice_hidden` so no second Vec is built.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
