# EditableText automatic viewport reveal

Four exact `widgets/editable_text_show_on_screen_test.dart` originals
passed against the pinned Flutter reference. The retained native
ListView cases open the keyboard with the editor half obscured and
verify offsets of 0 at the leading edge and approximately 150 with
a 200 px spacer and a 50 px inset. A focused editor scrolled out of
view remains editable and comes back when typing, without remounting.
With implicit scrolling disabled it remains off screen and still
accepts input. The last native case previously scrolled to offset 86
despite the policy.

Automatic caret reveal now calls the ordinary render ShowOnScreen
path with the padded local caret rectangle, instead of duplicating
ancestor viewport traversal. This respects implicit-scrolling
physics and retains pinned-sliver and nested-reveal behavior.
All 15 focused native cases passed. These checks cover the settled
viewport position, not Flutter's reveal animation duration.

## PageView and multiline input

Three more pinned originals passed. Typing on the middle page leaves
the page at index 1, with no active animation and no request for
another immediate frame. In a tall 40-line editor, entering text
at the off-screen end reveals its caret. Replacing that old caret
with the selection 26→27 there scrolls to exactly 28 px, not back to
the end or to a later line. The three focused native runtime tests
use real input events and the retained layouts.
