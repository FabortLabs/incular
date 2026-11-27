# Flutter full editing-state channel

The exact pinned `services/text_input_test.dart:259` passed. Flutter
rejects the incoming range 2–3 for text of UTF-16 length 1 before
delivering it. The Rust fixture mounts and attaches a focused editor
containing `test3`, sends the actual JSON method, and confirms that
the typed error names the range start (2) and the text length (1).
The controller value, focus and outgoing edit state remain unchanged.
Both the active real ID and Flutter's debug-only -1 path are exercised.

Native controls preserve reversed selection order across an astral
character, convert composition to byte boundaries, reject partial
sentinels, malformed metadata and range endpoints inside a surrogate,
and check stale client identity before decoding the state. Diagnostics
do not include the supplied editing text. A valid update uses the
existing platform-to-controller path, so it does not echo the same
state back to the host.

The byte-addressed editor represents Flutter's absent selection as a
collapsed end caret. The initial comparison only verified affinity and
directional wire types. The application of those flags is verified below.

## Retaining the visual edge through platform edits

The optional `TextInputClient.updateEditingState` adapter previously
discarded the supplied affinity and directional flag. It now commits both
with the UTF-8 selection and composing range in one controller change.
For the same text, collapsed status and selection start, Flutter keeps
the local visual edge instead of trusting an affinity-only change from
the platform. Otherwise it uses the incoming edge. Unrecognized string
values default to downstream, as in `TextEditingValue.fromJSON`.

The pinned authored `editor_platform_selection_affinity_test.dart`
sends the actual JSON channel messages to a focused editor. The native
tests reproduce both the ignored duplicate and a moved selection that
must use the new visual edge, preserve directional reverse selection
through an astral character, and verify no formatter for selection-only
changes, one formatter for the text change, no duplicate caret revision,
and no echo. The default native IME and delta adapter paths were not
changed. Four focused native targets passed 29 tests. This is authored
coverage and adds no upstream registration count.

## Physical keyboard selection callbacks

Without waiting for an IME JSON message, physical Right Arrow moves
the mounted editor from offset 2 to 3 and Shift+Right selects 3→4.
Both the pinned `editor_keyboard_selection_cause_test.dart` and retained
Rust dispatch report `Keyboard` once for each changed selection. Native
typed text replacing that range reports its resulting collapsed selection
as `Keyboard`; key release and direct controller setters do not add
notifications. The shared comparison does no selection snapshot work when no
callback is installed. Three focused native targets passed 27 tests.
No original registration count was added for the authored cases.

## Live interactive selection configuration

The pinned editable-text configuration test changes
`enableInteractiveSelection` from true to false on the mounted multiline
editor. The Rust regression verifies the same switch in the actual
keyboard command. It uses the same client ID, selection and text, does
not re-show or close the keyboard, and does not resend its unchanged
configuration on the next frame. All six focused native tests passed.

## View identity in mounted keyboard configuration

Pinned `widgets/editable_text_test.dart:1286`, “EditableText sends viewId
to config,” passes for view IDs 77 and 88. Incular's opt-in Flutter channel
now gets that ID from its host binding, not from a native window slot.
The native test starts with view 77, moves the same mounted editor to view
88, and checks the actual JSON `TextInput.setClient` configuration together
with the new multiline and selection policy. A separate runtime remains
on view 99.

A view-only update reissues configuration without changing the client's
ID, editor focus, selection, or keyboard visibility. A pending attachment
is not duplicated, and changing a view ID then reverting it before the
frame sends nothing. The default native path does not acquire a Flutter
ID. The test uses two retained runtimes and their protocol bindings, not
two OS window surfaces. Three focused native targets passed 14 tests.
