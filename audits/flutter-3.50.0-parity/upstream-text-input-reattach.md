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

Four original `widgets/editable_text_test.dart` registrations pair at
the runtime/host boundary, with one partial because automatic keep-alive
lifecycle is not yet paired. Closing the current host connection also
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

## Reopened connections after a tap or viewport eviction

The lazy-list original pairs. Once the host closes the connection, the
unfocused row can be evicted. Neither its old element ID nor the old host
connection can refocus it, even after it remounts. A direct focus request
for the new mounted editor succeeds and attaches a fresh client without
losing the draft.

Both tap-to-reopen originals ran. Incular matches the active input
client, focus, and editing-value transitions, but does not yet claim
Flutter's focus-driven automatic keep-alive policy. Flutter's
second-field fixture also shares one FocusNode between both fields;
the native control uses separate retained focus nodes and a shared
editing controller. These two cases and the previous close case that
asserts `wantKeepAlive` are therefore partial. Three pinned Flutter and
seven targeted Rust cases passed.

## Automatic keep-alive follows editor focus

The pinned authored Flutter control keeps the same editor state
and primary focus when its lazy row leaves the viewport cache. When the
host closes input, that row can be evicted without submitting, echoing
a host command, or changing the draft. Incular now routes a focused
editor's automatic retention through the materialized sliver ancestry.
It releases only that automatic request; an explicit KeepAlive
continues to retain the row. A non-editor lazy focus target still
detaches as before. The authored Flutter probe and 16 targeted Rust
cases passed.

Together with the close and native tap controls above, this completes
the single-editor close and reopen cases. The other-field original
still uses a shared FocusNode across two fields and remains partial.
The original that inserts a large item before a kept keyed editor
passed on Flutter only; keyed remapping plus offscreen repaint is still
a distinct comparison.

## Keyed editor rebuild with skipped layout

The previously open `editable_text_test.dart:2514` now pairs. The
pinned original and an additional Flutter control insert 6,000 px of
content before a focused keyed editor while changing alignment and
cursor visibility. The editor keeps its original state and focus
without changing the scroll offset or draft. The native regression
also checks the same input attachment persists and right alignment
takes effect when the editor returns to view.

Explicit ListView children now resolve a kept key's new index on
delegate replacement. The key map is built only if a kept keyed child
actually needs lookup. Both fixed and measured row paths carry the
automatic retention request only to the matching widget. The original
Flutter case, one authored Flutter case, and 18 focused Rust cases
passed. Builder callbacks for arbitrary lazy data remaps are separate.
