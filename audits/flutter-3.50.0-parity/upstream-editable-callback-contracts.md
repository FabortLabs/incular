# EditableText callback contracts

Reference: Flutter 3.50.0 (`ab2912035e160d9cad6306df307576da6128ebdb`),
`packages/flutter/test/widgets/editable_text_test.dart`. The matching Incular
target is `cargo test -p incular-runtime --test flutter350_editable_callback_contracts`.
The native cases mount a real autofocus editor and exercise pointer or platform
editing events, not callbacks invoked in isolation.

## Accepted changes and formatter boundaries

- **6056**: `onChanged` reports accepted text mutations. Merely committing a
  composing range or receiving a rejected text proposal does not report a change.
- **6100**: An identical packet or selection-only update does not run input
  formatters; distinct text does.

## Callback error recovery

- **12117**: A selection callback can fail on a tap without detaching the editing
  client. The observer receives the corresponding callback error and message.
- **12146**: A failing `onChanged` callback reports the error without undoing the
  text that was already accepted.
- **12240**: Flutter catches a failure around the formatter fold, rather than
  committing a partially formatted value. Incular reports the error and accepts
  the original unformatted edit and selection. Before this correction, the
  formatter and change callback failures both escaped the platform event.

Flutter reports an exception through its framework error channel; Incular
exposes a typed error through `Runtime::observe_text_editing_error` under Rust's
unwinding panic strategy. Controller edits remain distinct from callbacks
invoked directly by application code.
