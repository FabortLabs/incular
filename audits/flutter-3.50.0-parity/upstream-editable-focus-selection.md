# EditableText selection when receiving focus

Pinned Flutter 3.50.0 `ab2912035e`. Five registrations in
`widgets/editable_text_test.dart:17330,17552,17581,17613,17645` were run
in every native-runner platform variant (27 invocations).

- Desktop Tab into a single-line field selects all; on Android and iOS
  it initializes an unset caret and keeps a moved caret on revisiting.
  A multiline field keeps its caret on both. The Rust port uses actual
  caret geometry for the two repositioning taps. Before the repair,
  desktop Incular collapsed at the end instead of selecting all.
- An explicit false override keeps the original caret even on desktop;
  true selects all on either platform category. Clicking an editor
  itself still chooses a caret rather than selecting its entire text.
- EditableText's own autofocus is internal. It initializes the caret,
  not an all-text selection. Inactive/resumed retains that selected
  field and caret; a later Tab to another field still selects all
  on desktop.

Rust asserts the portable policies for Linux, Windows, macOS, Android
and iOS, using the same mobile behavior Flutter exercises for Fuchsia.
No Fuchsia host or browser document-root traversal test ran. The
reference uses the original app fixture, but only EditableText focus
and selection are in scope. Forty-five focused Rust checks across
editor, touch, keyboard, platform text input and initial-focus paths
passed. The independent programmatic invalid-selection refresh case
and OS window focus interception remain separate.
