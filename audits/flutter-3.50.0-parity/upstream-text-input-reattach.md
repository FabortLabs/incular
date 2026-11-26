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

## Host closes the connection and requests focus again

Five original `widgets/editable_text_test.dart` registrations pair at
the runtime/host boundary. Closing the current host connection also
ends the editor's focus. It does not submit, complete editing, change
committed text, or send Hide/Clear back to a host that already closed
the connection. An eligible mounted editor can regain focus on a
matching host request, including a retained Offstage editor. An
unmounted or no-longer-eligible editor cannot.

Client identity now denotes a **single attachment**, separate from the
element identity used for mounted autofill tags. Refocusing assigns a
fresh ID, so delayed close and focus messages cannot disturb the same
editor's new connection or a different focused field. The existing
state-resend path still reuses the current attachment. Both the optional
Flutter JSON adapter and the normalized host-close event were exercised.
This does not imply that each operating system's default adapter
generates these events for the same native interactions.

Five pinned Flutter and 32 targeted Rust cases passed, including
autofill, editing/IME gating, and client-state replay controls.
