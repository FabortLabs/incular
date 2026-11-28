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

### Keyboard appearance on the attached input client

Pinned `widgets/editable_text_test.dart:6137,6809` and the native
attached client carry `Brightness.light` by default and
`Brightness.dark` when explicitly requested. Native also checks
that the corresponding Show and subsequent editing-state update
use the attachment's same client ID. This compares the platform
configuration payload, not the operating system's rendered theme.

### Pointer cursor follows the editor before the containing MouseRegion

Pinned `widgets/editable_text_test.dart:11405` passed. An editor inside
a Forbidden mouse region uses its explicit Click cursor, then changes
to the default Text cursor on rebuild without any pointer movement.
Previously the containing region was consulted first, so even an
explicit child override appeared Forbidden. Native now evaluates the
editor in the retained hit order, distinct from activating the text
caret. It retains the editor and focus state across the update; an
additional explicit Defer case correctly exposes the parent's cursor.

All 74 focused cursor, hover, text-span, tap-outside and editable
interaction tests passed. Physical mouse handling was represented
with normalized platform events.

### Live keyboard configuration and initial style wire fields

Pinned `widgets/editable_text_test.dart:3815,6396,6439` passed.
Changing the type from Text to Number while editing `Lorem` keeps
the same focus and client. Its host control publishes exactly one
`TextInput.updateConfig`, without clearing or showing the keyboard
again.

The right-to-left, 20 px, weight-600 style reports the expected
Roboto family, index 5, start alignment, RTL direction and null
spacing. Bold accessibility changes only the weight to index 6.
Flutter's test host falls back to the embedded FlutterTest face,
so native uses that exact face under the advertised family name.
With the same face the line height is 20; the runner's unrelated
system fallback has different metrics and is not counted.
All six focused native style and client-update tests passed.

### Read-only keyboard actions

Pinned `widgets/editable_text_test.dart:7987` passed on all six target
platform variants. The mounted native editors keep the preset half
selection and text when Paste is pressed. Select All is still
allowed and emits one selection callback. Cut does not replace
the clipboard, Copy sends the entire document, and Delete and
Backspace leave the text and full range untouched. Native uses
the corresponding Ctrl and Command shortcut tables, with
Fuchsia sharing the Android path, and an in-memory clipboard.

### Restyling focused and previously focused fields

Pinned :6484 and :1772 passed. Rebuilding the active field changes
the wire style from RobotoMono to Raleway, weight 600 to 700,
letter spacing 1 and word spacing 2. The same client receives one
style update, and a repeated identical frame emits nothing.

An authored pinned reference and the mounted native fixture also
show the earlier field relayout from 9 to 20 px after its controller
and style are replaced, while the current field stays 9 px and
retains focus. Native uses separate focus nodes, as does the authored
reference, so :1772 is partial rather than claiming equivalent
behavior for the original's shared FocusNode attachment. The eight
focused native tests used the matching embedded FlutterTest font.

### Paste replaces the selection and lands after the inserted text

Pinned :14881 passed. The two native paths use the keyboard and the
neutral toolbar action. Starting from typed `12345`, five Shift-Left
events form the reversed (5,0) range, then Copy and Paste restore
the same text with the caret at 5. Replacing forward range (4,5)
yields `123412345` with the caret at 9. Selecting all and pasting
the shorter value returns `12345` and caret 5. The test compares
the retained editing action, not Material toolbar styling.

## Multiline size notification

Pinned `widgets/editable_text_test.dart:18171` passed. A mounted native
multiline editor with limits 1–3 receives a newline through the normal
platform editing event. Before the fix the editor changed height but
its enclosing listener received no notification. It now receives exactly
one `SizeChangedLayoutNotification` when the height changes and none
on mount, focus, or a subsequent unchanged frame. Width and retained
focus identity stay unchanged. The focused native regression passed.

## Duplicate toolbar request after autofocus

Pinned iOS `widgets/editable_text_test.dart:18516` passed. A neutral
native toolbar starts hidden after autofocus. The host's first request
returns true and mounts it. A second request returns false and preserves
the exact same retained overlay across subsequent frames. The same
test fixture still verifies this during a handwriting interaction.
All seven focused binding and five font-pinned keyboard geometry tests
passed.

## Editor activation keys do not activate ancestor controls

Pinned `widgets/editable_text_test.dart:15021` passed. The native
mounted check initially fired the enclosing ActivateIntent action
when the focused target was an editor. EditableText now shadows both
generic and button activation while leaving Space on the native text
input path and preserving desktop newline input from Enter and
keypad Enter. An outer activation is never invoked for those editor
inputs, and ordinary focused controls still receive activation.
All four application activation and 19 editing shortcut tests passed.
