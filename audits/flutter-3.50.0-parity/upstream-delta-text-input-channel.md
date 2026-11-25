# Services: opt-in delta text-input channel

All five exact `services/delta_text_input_test.dart` registrations
passed on pinned Flutter 3.50.0. The native tests pair `:29` through a
real encoded `TextInput.setClient` and `TextInputClient.updateEditingStateWithDeltas`
round trip at the embedder boundary. Existing text-input configuration
fields are retained and `enableDeltaModel` is set only for an explicit
`FlutterDeltaTextInputConnection`.

The `:65`, `:115`, `:161` and `:214` controls use the original invalid
payloads. Rust reports the failing selection, replacement or composing
field and its delta index; no part of the batch is delivered. Flutter
reports debug assertions with localized editing text in its diagnostics;
the Rust errors are typed and omit that text. Stale-client and
non-delta method messages do not invoke the callback, and the Flutter
test-only wildcard ID is ignored in release builds.

Four focused Rust tests passed. This is an opt-in API for a Flutter
protocol embedder, not a change to the default winit, Android, iOS, or
Web IME path.
