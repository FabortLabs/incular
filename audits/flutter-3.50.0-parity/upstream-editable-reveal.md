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

## Read-only focus and caret repaint

Both read-only expansions of two pinned registrations passed. A
field whose existing caret is off screen can receive focus even
when read-only. Focus alone then leaves both the editor's horizontal
scroll and its enclosing list unchanged. The corresponding editable
field scrolls both to reveal its pre-existing caret. Native behavior
previously revealed the read-only field too. The guard is limited
to initial reveal, so later selection changes within a focused
read-only field can still scroll.

A separate native fixture also advances actual blink boundaries
after explicitly scrolling the focused editor horizontally, verifying
that paint activity never snaps the scroll back. All 24 focused native
caret, focus, and reveal cases passed.

## Selecting through the native keyboard and selection delegate

Both read-only and editable expansions of both pinned registrations
passed. A selection-only update from the native keyboard moves the
selection and reveals the caret in an editable field, but is ignored
by a read-only native field. The field's own selection delegate
can reveal the caret even when read-only. The native fixtures reset
both scroll positions before each update and check both positions,
the unchanged text, the selection, and retained focus. The same
results hold with a formatter which would reject replacement text.
Both focused native tests passed. This completes all 14 registration
sites in the pinned show-on-screen test file.
