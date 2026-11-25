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
passed. OS window focus interception remains separate.

## Programmatic text replacement with an invalid selection

Original `widgets/editable_text_test.dart:17439` passed: after focus was
established, replacing the text synchronously moves an invalid mobile
selection to the end. Incular previously left it unset. An authored
neutral Flutter probe also checks desktop select-all and absence of
repair after unmount. Rust additionally verifies that replacing the
controller in a still-focused editor transfers the policy.

Eighteen focused runtime checks, ten general controller checks, and
seven value replacement-boundary checks passed. Two assertions in
`editing_copies` still conflate caret collapse after a controller
user edit with value-level range remapping; both failures were reproduced
on the unchanged baseline. No browser or native host was required.

## Obscured code-point movement and deletion

The pinned obscured-editor registration :17767 and the retained Rust
keyboard comparison agree on every arrow and Backspace. The family
emoji's UTF-16 caret positions are 8→6→5→3→2→0 and back. Backspace
then removes each code point, including each zero-width joiner,
instead of treating the family as one grapheme. The Rust assertion
converts actual UTF-8 caret positions to UTF-16 only for comparison.

The related pinned :17872 passed on Flutter but is not paired: Dart
can place a caret between the two UTF-16 surrogates of one scalar,
while the portable Rust editor uses valid UTF-8 boundaries. A
separate authored desktop probe confirmed that programmatic text
replacement selects the entire value; Backspace there deletes all.
With an explicitly collapsed caret, the unobscured editor instead
removes only the emoji grapheme. The older Rust repeat test now
sets that caret before checking its grapheme expectation.
The two focused Rust comparisons and the corrected repeat test passed.

## Default mask geometry and accessibility

Flutter's default mask displays one bullet per UTF-16 code unit.
Before the repair the portable editor shaped 13 asterisks for
`é👩‍💻`, because its model uses UTF-8 bytes. It now shapes six
bullets, projects caret and selection positions between the two
coordinate spaces, and maps pointer hits back to valid source
boundaries. The equivalent plaintext bullet run matches the
obscured run's glyphs and caret rectangles in each framework.
A partially composed value also stays masked without replacing the
committed source text. An attempt to hit inside a surrogate pair
still resolves to a valid scalar boundary in Rust.

The retained accessibility value is now masked too, including
when toggling obscuring on an existing field. Native accessibility
projections still suppress password values; the editor controller
and input client keep their unchanged source value. Two pinned
originals (:5423,:5482) ran, and the Rust checks match the obscured
value and its retained toggle. Those originals remain partial:
their whole semantic trees include input-type and cursor-movement
actions not yet compared. The authored pinned probe adds password
glyph and caret evidence without counting a new original.

Forty focused Rust tests passed, including the retained toggle,
platform text input, spell checking, and keyboard deletion.

## Configurable obscuring character

The pinned :5607 original now pairs. The editor accepts a typed
mask character and shapes 24 hashes for the original 24-character
ASCII fixture. An authored Flutter control and Rust counterpart
also replace the mask on a retained editor, using a non-ASCII BMP
glyph for a six-unit password. Glyphs, projected caret, a pointer
hit, and the masked semantic value continue to agree with the
selected mask, without changing the controller text. A supplementary
character is rejected since Flutter requires exactly one UTF-16 unit.
Both authored cases and 45 focused Rust tests passed.
