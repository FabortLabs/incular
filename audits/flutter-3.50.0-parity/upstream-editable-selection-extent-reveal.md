# Reveal a selection box, not just its caret

Pinned Flutter 3.50.0 `EditableTextState._scheduleShowCaretOnScreen`
(`widgets/editable_text.dart:4613`) uses the first selected box for a
backward selection and the last selected box for a forward selection.
A collapsed selection, or a noncollapsed selection with no complete
box, still uses the caret. Scroll padding is applied after choosing
the target.

The pinned Linux probe `editable_selection_extent_reveal_test.dart`
and the retained native `flutter350_editable_selection_extent_reveal`
test cover a horizontal 100-pixel viewport. A same-run selection that
already spans the viewport leaves its offset at 0. A narrower forward
selection whose right edge is 180 scrolls to exactly 100 after adding
20 pixels of padding; the former native caret target scrolled to 102.
A backward selection at the leading end scrolls back to 0 and does not
lose focus. The fixture uses fixed Ahem advances and does not compare
goldens. It is authored, so it does not increase original-test coverage.

The native geometry uses retained shaped clusters and walks only the
selected lines nearest the active edge, without allocating the full
selection's box list. This fixture verifies the common single-direction
case; bidi box order, nondefault selection styles, explicit line breaks,
and partial-cluster behavior are not independently verified here.
Four adjacent grouped-handle and keyboard-inset tests also passed.
