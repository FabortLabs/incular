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

## Lazy ListView key and item index lookups

The pinned `list_view_test.dart:81,115` originals pair for builder and
separated lists. The supplied callback receives the previous live key
when the delegate is replaced, even when the row is not kept alive and
the callback returns no new index. It does not run on initial layout
or normal scrolling. Native callbacks use the immutable keyed widget's
original key. The compatibility child-index hook counts separators;
the item-index hook maps item indices to the even child slots.

An authored Flutter control moves a focused editor from item 0 to 90
while the viewport stays at zero. The callback preserves its state,
text, and focus even though it now needs to park outside the viewport.
Closing the host connection releases the kept row. Both originals,
the authored control, and 20 focused Rust tests passed. This does not
claim that all sliver classes expose these ListView hooks.

## Do not replay composing ranges after blur

Two authored Flutter cases pair. A keyboard close or ordinary focus
transfer ends the old composing range without changing text or the
caret. When the same draft controller is shared with a phone field,
its fresh client receives the existing draft and an empty composing
range. Closing an already-retired client stays silent.

Incular previously cleared only native preedit overlays, leaving an
IME range supplied as part of a whole editing state active. Blur now
clears that committed controller range before taking the next client
snapshot. Composing-only operations preserve an unset selection and
do not notify when there was nothing to clear. Twenty runtime/host
tests and ten controller/state tests passed; these authored cases
do not add pinned registrations. The shared-FocusNode fixture above
still has a separate ownership difference.

## Moving outside composition

Pinned `widgets/editable_text_test.dart:11647,11670,11719` pass. The
controller formerly kept composition even when the caret or selection
left it. A moved or explicitly assigned selection now preserves the
range only when both endpoints are inside it, including exact
boundaries and backward selections. Moving outside changes neither
the text nor the selected offsets.

Native checks preserve all pinned inside/before/after positions, add
backward and partially overlapping selections, and verify the same
mounted client receives its updated selection and composing range
without a new attachment or keyboard show. Thirteen focused text
tests and 21 runtime/host tests passed.

## Keyboard close restores scope focus

An authored Flutter control focuses two editors successively in one
scope, then closes the second client's keyboard. The nearest scope
itself takes primary focus, its child history is cleared, and neither
editor becomes active. No hide or clear command is echoed. Requesting
focus again attaches the first editor to a fresh client with its own
draft. Previously Incular cleared primary focus entirely even with a
mounted scope. The native scope counterpart plus 19 focused lifecycle
and focus-scope tests passed. The scope test is authored.

## Platform lifecycle requests and the focused editor

The three services originals at :777, :799, and :826 ran successfully.
They use an injected fake client to record callback dispatch, including
a focus request while a connection is still active. Incular instead
routes wire messages to its mounted editor; it does not expose that
arbitrary fake client interface. The originals are therefore partial,
not counted as paired.

The authored pinned mounted-editor control and all 11 focused Incular
tests pass. While already focused, the mounted editor declines the
request without a second attach or show. Closing leaves the committed
text unchanged and sends neither submit nor completion. The eligible
editor can subsequently refocus with a fresh client ID, and the old ID
cannot refocus or close that new connection. Flutter can also refresh
its caret geometry during the active frame; the control distinguishes
that from a connection or visibility change.

The services selector-delivery original :876 also passed in Flutter.
Incular has no corresponding selector delivery route yet.
