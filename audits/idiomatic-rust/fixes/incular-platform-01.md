# Fix brief incular-platform-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-platform/src/data_transfer.rs`, `crates/incular-platform/src/file_dialog.rs`. Other agents are editing other files at
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
### A113-01 - crates/incular-platform/src/data_transfer.rs:201 (§2, low)
Evidence:
```
    provider: Arc<dyn Fn() -> Result<TransferData, TransferDataError> + Send + Sync>,
    ...
                provider: Arc::new(provider),
```
Suggested fix: The provider is only ever called through `(self.provider)()` and the whole LazyTransferData is already shared via Arc<LazyTransferData> in TransferPayload::Lazy, so the inner Arc is a redundant second heap allocation per lazy representation. Change the field to `provider: Box<dyn Fn() -> Result<TransferData, TransferDataError> + Send + Sync>,` (line 201) and the constructor to `provider: Box::new(provider),` (line 307 in TransferRepresentation::lazy).

### A113-02 - crates/incular-platform/src/data_transfer.rs:367 (§2, low)
Evidence:
```
            if !formats.insert(representation.format.clone()) {
                return Err(TransferDataError::DuplicateFormat(
                    representation.format.clone(),
```
Suggested fix: The HashSet is only used for membership, so it can borrow from `representations` instead of cloning each format (a Custom format clones a String per representation): `let mut formats = HashSet::with_capacity(representations.len());` and `for representation in &representations { if !formats.insert(&representation.format) { return Err(TransferDataError::DuplicateFormat(representation.format.clone())); } }`. The borrow ends before `representations.into()` (NLL), so the rest of the function is unchanged.

### A114-02 - crates/incular-platform/src/file_dialog.rs:193 (§6, low)
Evidence:
```
let Some(kind) = parts.next() else {
    unreachable!()
};
```
Suggested fix: Replace the split/next chain with split_once: let Some((kind, subtype)) = trimmed.split_once('/') else { return Err(FileDialogValidationError::InvalidContentType(original.to_owned())); }; then reject subtype.contains('/') with the same error, removing the unreachable!() panic path.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
