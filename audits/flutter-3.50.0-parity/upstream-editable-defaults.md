# Raw EditableText visual defaults

Pinned Flutter 3.50.0 `widgets/editable_text_test.dart:1395` passed.
The native mounted control keeps a genuine selected range and focus
while inspecting the actual draw commands. A raw editor paints no
selection highlight by default, including under an orange
DefaultSelectionStyle, and it does not manufacture a focus border.
Supplying an explicit selection color paints it; clearing that color
removes the paint without moving focus or changing the selected range.

The styled Incular controls retain their existing highlight by
requesting it explicitly. Ordinary and spell-check selection colors
are independent; the spell-check toolbar checks still pass.

The control checking absence of a focus border is authored and does
not add a second upstream registration.

The broader check reproduced two stale alignment expectations on
the prior commit, independently of the color change. The Flutter
source reserves the 2 px default cursor plus 1 px of caret gap; the
native single-line LTR tests now account for the existing reservation
instead of comparing with the whole area. RTL fixture expectations
outside that native margin path are unchanged in this batch.
All 48 focused native controls pass, including color, spell-check,
and geometry. No motion or platform text policy changed here.

## Logical selection on focus and blur

The pinned Linux reference at `widgets/editable_text_test.dart:1344`
and the native port both pass. The initially unfocused editor keeps
the (5, 7) range and upstream affinity, then keeps them both while
focused and after blur. The comparison uses the original's native
branch; its conditional web select-all path is not newly verified here.

## Caret space in both directions and line modes

The authored pinned Flutter probe uses an empty editor so no font
advance affects the result. For width 200 and a two-pixel cursor
with one pixel of caret gap, the actual left, center, and right
caret positions are 0, 98.5, and 197 at device-pixel ratio 2.
Both line modes and both text directions match. At device-pixel
ratio 3 Flutter additionally snaps the midpoint to a device pixel;
that separate snapping rule is not part of this probe.

Before the native fix, both multiline modes and single-line RTL
used the full width and placed the center at 100 and right at 200.
The real layout width now reserves the cursor's requested width
and gap when aligning or wrapping, rather than special-casing
single-line LTR. Native tests inspect painted rectangles and
verify Bidi, pointer, placeholder, and scroll regressions.
All 72 focused native checks passed. This authored control does
not increase the upstream count.

## Caret and selection paint order

Three exact originals from `rendering/editable_test.dart` passed.
A noncollapsed selection draws exactly one highlight behind the
text. A retained cursor can switch between below and above glyph
paint; this changes command order but not the selection. Clearing
the selection, or setting the text to its invalid-selection state,
does not paint the caret or suppress the text. Restoring a valid
collapsed selection brings the caret back.

The native fixture compares the actual retained display-list order
and the number and color of rectangles. All 33 focused native
checks passed. The raw editor still defaults to painting its caret
below the text, and the renderer remains wgpu.

## Retained alignment invalidation

Pinned `rendering/editable_test.dart:334` passed. The native graph
starts at default Start, is quiet on a repeated layout without
changes, and schedules one real layout after Center is set. The
same render object survives, the painted glyphs move to the
center, and subsequent identical layouts are quiet. All nine
focused alignment controls passed.

## Default input semantics and single-line geometry

Pinned `rendering/editable_test.dart:144,159` pass in both engines.
An unfocused, undecorated editor advertises a text input type without
an explicit keyboard or semantic override. With the pinned Ahem font at
10 px, the default single-line viewport measures exactly 10 px even
when the controller holds ten newline-terminated lines.

The pinned :200 comparison remains partial. The exposed collapsed
handle moves from (0, 20) to (100, 220)
when the editor's origin moves by (100, 200); this is actual retained
selection geometry, not Flutter's pair of internal LeaderLayers.
All four direct native tests and two adjacent handle-reveal tests
passed with the pinned font, including the repository-checkout fallback.


## Actual height of a taller styled child

Pinned `rendering/editable_test.dart:125` passes. The retained native
editor now measures the original `TALL` text, a 100 px child span
under a 10 px outer style, to exactly 100 px with the low-level
renderer's unforced strut and default single-line width cap.

The native fixture supplies the child through the editor's real
spell-check style path. Before the fix an unset selection was mistaken
for a real caret at the end of the final word, and its child style was
dropped; both the pinned authored spell-check probe and native test
now distinguish 10 px with a real end caret from 100 px with no
selection. Fourteen adjacent spell-check, composing, and fixed viewport
checks also passed.

## Inherited and explicit editor line height

The pinned widget cases `:657`, `:11543`, and `:11571` passed.
A raw retained editor now resolves an explicit TextHeightBehavior
before its inherited DefaultTextHeightBehavior and leaves both
unset in the default state. The resolution feeds paragraph
measurement, not just the widget property. A one-line editor uses
the measured paragraph height rather than restoring trimmed leading
through a generic line-height minimum.

Both the authored Flutter probe and the native fixture use the
same embedded FlutterTest face with a disabled strut. Retained
updates produce heights 25, 20, 40, and 25 px as the parent rule
and explicit overrides change. The ordinary editor still
advertises Text input with autocorrection, suggestions and IME
personalization enabled; locale hints are absent. Its cursor is
2 px wide with no explicit height, text is unobscured, and LTR
start remains flush left. All 13 native checks passed.

## Native keyboard action and privacy hints

Eighteen pinned widget registrations passed: the default keyboard,
13 explicit-action cases (two original registrations cover Send),
the explicitly multiline and visible-password keyboards, and
the opt-outs for suggestions and IME personalization. The native
fixture attaches and shows the real runtime client, updates its
controller to `test`, then compares the configuration that the
same client gives to the Flutter-format bridge.

An explicit Unspecified action previously became Done or Newline.
An omitted action also incorrectly followed the editor's layout line
count even when its keyboard type was overridden. Unspecified now
reaches the host unchanged, and Automatic chooses Newline only for
the multiline keyboard. This applies to both the ordinary client
and autofill serialization. The styled field retains its previous
line-based fallback; this change does not audit design systems.
The privacy flags are checked separately, so turning off either one
does not turn off the other. Forty-two focused native checks passed.

### Keyboard type after a pointer actually focuses the editor

The five pinned EditableText registrations at :1864, :1900, :1937,
:1974 and :2008 passed. Native tests start with no focused editor or
client, dispatch a real pointer down and up, and inspect the attached
client's configuration and Show command. Controller text changes to
`test` reach that same client. Unlimited and three-line editors
default to the multiline keyboard and Newline. An explicit Text
keyboard on the unlimited editor and Phone on the three-line editor
both use Done. The untouched one-line editor uses Text and Done.
All 17 focused tests passed. This compares real runtime keyboard
requests and wire payloads, not rendering of the host keyboard.
