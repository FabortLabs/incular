# Pointer-owned editor selection callbacks

Pinned `widgets/text_selection_test.dart:829`, “test
TextSelectionGestureDetectorBuilder double tap,” passes. The retained
editor now reports `DoubleTap` when two touch presses select the word and
opens its configured context toolbar. The test drives real host pointer
events; it does not call the selection callback or toolbar directly.

The pinned authored `editor_pointer_selection_cause_test.dart` delegates
gestures to Flutter's `TextSelectionGestureDetectorBuilder` and disables the
raw renderer's competing recognizers, as Flutter documents for that
configuration. Both fixtures select offset 2 on click (`Tap`), range
0→5 on double tap (`DoubleTap`), range 1→8 on a desktop mouse drag
(`Drag`), and range 6→10 on a touch long press (`LongPress`). The native
wiring also reports the selection changes made by long-press moves,
secondary clicks and selection handles on their current routes. It does
not change the selection algorithm, focus policy or what the menu draws.

Three focused native targets passed 25 tests. Only the exact original
double-tap registration increases the audit count.

## Accessible editing and toolbar selection

The pinned `editor_semantic_selection_cause_test.dart` finds the actual
semantics node that advertises SetSelection. Both retained editors report
`Keyboard` for an accessible range change, extending that range by one
character, and replacing the text. The toolbar's Select All reports
`Toolbar`, and focus is retained. Native menu intents also carry that
cause when they change the selection.

One older native test asserted that an unfocused, disabled field
advertised SetSelection. The same assertion failed at the unmodified
baseline. The corrected fixture checks Flutter's focused-only action
advertisement, while preserving Incular's direct native SetSelection
compatibility and rejecting a direct SetText on that field. Three
focused native targets passed 18 tests. These authored cases add no
upstream registration credit. Ordering when both onChanged and
onSelectionChanged are installed remains a separate comparison.
