# Host requests the existing text-input connection

Pinned originals `services/text_input_test.dart:205` and `:226` both
pass. Flutter only handles a replay while a connection is active. It
resends the same client ID and configuration, followed by the current
editing value; it does not show the keyboard again. Despite its name,
the second test supplies a non-null, empty `TextEditingValue`. Flutter
serializes its unset offsets as -1. Incular's focused client represents
the empty selection as a caret at 0, and no composition as `None`. These
are not claims of wire-format equivalence.

Incular previously had no event by which the host could request this.
`PlatformEvent::RequestExistingTextInputState` now emits one
`SetClient` carrying both pieces of data (the Incular command already
combines them) without a `Show`. The focused Runtime tests verify both
original text cases, that no client means no reply, and that a
programmatic edit made since the last frame is replayed with its
reversed selection and composing range. The idle following frame does
not resend the value. This is verified at the normalized host boundary;
native adapters' behavior upon OS-specific reconnects was not tested.
