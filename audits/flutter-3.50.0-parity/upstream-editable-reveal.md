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

## Large insets and pinned header editors

Three exact originals passed. The bottom inset can be as tall as the
300 px viewport; opening the keyboard then moves the preceding
200 px row completely out of view. Focusing a field in a 50 px
persistent header pinned at scroll offset 1500 no longer rewinds the
outer viewport to 1480 to accommodate the caret's 20 px padding.
The pinned header trims the requested rectangle at its growth-leading
edge, independently of the floating-header expansion option.

The Rust cases retain focus and leave offset 1500 unchanged for both
timed and immediate ShowOnScreen paths, with no running viewport
animation. The two upstream cases differ in whether the delegate
supplies vsync; the Rust layout uses explicit request durations
instead of an external ticker provider. All 14 focused native
reveal and pinned-sliver checks passed.
