# Fix brief incular-accessibility-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-accessibility/src/lib.rs`. Other agents are editing other files at
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
### A002-01 - crates/incular-accessibility/src/lib.rs:679 (§4, low)
Evidence:
```
changed.sort_by_key(|id| self.semantic_to_native[id].0);
```
Suggested fix: changed.sort_unstable_by_key(|id| self.semantic_to_native[id].0);  // keys are unique native ids, so unstable sort gives the same deterministic order without the stable-sort buffer

### A002-02 - crates/incular-accessibility/src/lib.rs:857 (§10, low)
Evidence:
```
if self.semantic_to_native.contains_key(&node) {
    return true;
}
```
Suggested fix: let Entry::Vacant(entry) = self.semantic_to_native.entry(node) else { return true; };
let native = NodeId(self.next_native_id);
let Some(next_native_id) = self.next_native_id.checked_add(1) else { return false; };
self.next_native_id = next_native_id;
entry.insert(native);
self.native_to_semantic.insert(native, node);
true  // Entry from the map's std::collections module (hash_map or btree_map, matching the field type); disjoint field borrows keep this compiling


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
