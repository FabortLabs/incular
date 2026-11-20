# Fix brief incular-devtools-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `tools/incular-devtools/src/transport.rs`. Other agents are editing other files at
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
### A041-04 - tools/incular-devtools/src/transport.rs:123 (§7, low)
Evidence:
```
let bytes = serde_json::to_vec(&request).map_err(|_| ClientSendError::Oversized)?;
```
Suggested fix: Add a distinct ClientSendError::Encoding variant and map the serde failure to it instead of Oversized. Because the enum is Copy, the serde message cannot be stored in it; if it must be kept, have send() return the message alongside the variant and pass it to note_request_error in send_or_report.

### A041-05 - tools/incular-devtools/src/transport.rs:123 (§2, low)
Evidence:
```
let bytes = serde_json::to_vec(&request).map_err(|_| ClientSendError::Oversized)?;
if bytes.len() > REQUEST_MESSAGE_LIMIT {
```
Suggested fix: Measure the encoded length without allocating: serialize into a counting io::Write (for example a small struct that adds buf.len() to a usize in write/flush) and use that count for the size check and RequestBytePermit. This removes the throwaway Vec. Check that the request encoding in send_request (outside this range) does not serialize it a second time.

### A041-06 - tools/incular-devtools/src/transport.rs:497 (§7, low)
Evidence:
```
let Ok((websocket, _)) = tokio_tungstenite::connect_async_with_config(
return retry_outcome(model, "connection refused");
```
Suggested fix: let (websocket, _) = match tokio_tungstenite::connect_async_with_config(format!("ws://127.0.0.1:{}/", record.port), Some(websocket_config), false).await {
    Ok(connected) => connected,
    Err(error) => return retry_outcome(model, format!("connection failed: {error}")),
};

### A042-01 - tools/incular-devtools/src/transport.rs:760 (§2, low)
Evidence:
```
state.target_info = Some(info.clone());
state.windows = info.windows.clone();
```
Suggested fix: `info` is owned here (`let info = *info;` at line 747). Keep one windows clone and move `info` into target_info last, after the console message that reads info.executable and info.platform: `state.windows = info.windows.clone();` ... `state.push_console("info", "devtools", format!("connected to {} ({})", info.executable, info.platform));` then `state.target_info = Some(info);`. This removes a full TargetInfo clone including a second copy of the window list.

### A042-05 - tools/incular-devtools/src/transport.rs:1178 (§8, medium)
Evidence:
```
if message.contains("recording stopped") {
    state.recording = false;
    state.error = Some(message);
```
Suggested fix: Do not infer recording state from the text of a log line. Carry the fact in the event: add a `TargetEvent::RecordingStopped` variant (or a reason field on the existing Log event, for example an enum LogKind { Plain, RecordingStopped }) in incular-devtools-protocol, emit it from the target, and set state.recording = false in its own arm. This is a protocol change, so record it in the CHANGELOG if the wire format changes.

### A042-04 - tools/incular-devtools/src/transport.rs:1246 (§26, low)
Evidence:
```
    }
    let _ = request;
    notify(
```
Suggested fix: Delete the line `let _ = request;`. `request` is already read by the arms above, so the statement silences nothing and only adds noise. If the compiler does warn about an unused binding, bind it only where it is used instead of at function scope.

### A042-06 - tools/incular-devtools/src/transport.rs:1315 (§2, low)
Evidence:
```
let request = Message::Request {
    request_id,
    body: body.clone(),
```
Suggested fix: Serialize a borrowed wire form so the body is not cloned just to be encoded. Define a private `#[derive(serde::Serialize)] struct RequestFrame<'a> { request_id: u64, body: &'a RequestMethod }` that serializes with the same field names and tag as Message::Request (check the protocol's serde attributes), encode that with `serde_json::to_string(&RequestFrame { request_id, body: &body })`, then move `body` into PendingRequest. The clone at line 1315 is then gone. Keep the wire format identical and verify with the protocol round-trip tests.

### A042-03 - tools/incular-devtools/src/transport.rs:1317 (§7, medium)
Evidence:
```
let text = serde_json::to_string(&request).map_err(|_| ())?;
if text.len() > REQUEST_MESSAGE_LIMIT {
    return Err(());
```
Suggested fix: Stop dropping the source error: change send_request and send_internal_request to return a small private error enum (for example SendError { IdsExhausted, Encode(serde_json::Error), TooLarge, TooManyInFlight, Closed }) instead of Result<(), ()>, map the serde error with `.map_err(SendError::Encode)?`, and report the reason in the console or tests. Callers that currently use `let _ =` or `.ok()` keep compiling. The same Result<(), ()> is also used for the overflow check at line 1312 (`ok_or(())`).

### A042-02 - tools/incular-devtools/src/transport.rs:1366 (§2, medium)
Evidence:
```
fn payload_fits(value: &impl serde::Serialize, limit: usize) -> bool {
    serde_json::to_vec(value).is_ok_and(|payload| payload.len() <= limit)
}
```
Suggested fix: Measure without allocating the whole serialized payload: add a tiny io::Write that tracks remaining budget and fails when a write exceeds it, then `fn payload_fits(value: &impl serde::Serialize, limit: usize) -> bool { serde_json::to_writer(BudgetWriter { remaining: limit }, value).is_ok() }`. BudgetWriter::write returns Err when buf.len() > remaining, else subtracts and returns Ok(buf.len()); flush returns Ok(()). Callers passing SemanticsTree, LayerTree and Signals payloads then never build a Vec of the full JSON.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
