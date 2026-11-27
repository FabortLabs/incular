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
upstream registration credit. The order when both callbacks are
installed is compared separately below.

## Caller-owned pointer handling

Flutter's `rendererIgnoresPointer` lets a caller provide text gestures
without competing with the renderer. The pinned authored probe verifies
that a direct tap does not move the selection or take focus while that flag
is on. It leaves position queries and explicit `selectPositionAt` available,
then restores regular tap selection when the live renderer is reconfigured.

Incular adds `EditableText::renderer_ignores_pointer` (false by default)
and gives explicit selection calls a typed cause, without requiring a
different renderer or adding layout borders. The mounted renderer and
focus identity stay stable across the toggle. The two native editor
cases and 12 adjacent tap and gesture checks passed. This authored
comparison adds no original registration count.

## Selection callback before text callback

The pinned `editor_change_callback_order_test.dart` sends a real full
editing-state update to a focused editor with both callbacks. Flutter
reports the new selection before `onChanged`. If the selection callback
restores the previous text, there is no false text-change notification.

The native regression reproduced the reverse order. Incular now defers only
the user text-change notification until selection callbacks finish, without
deferring committed controller state or ordinary listeners. It keeps the
already allocated previous string, so it need not copy large documents to
compare the final value. The native case checks platform and directly typed
edits, a re-entrant restoration, and the corrected value sent back to the
input method. The same ordering applies to the routed key, toolbar and
accessibility edit paths. Four focused native targets passed 34 tests.
No original registration count was added for the authored case.

## Android single-finger tap timing

The pinned pointer probe confirmed a focused Android editor keeps its
old caret through pointer-down and through a canceled tap. A completed
tap then moves it and reports exactly one `Tap`. Incular's regression
originally moved the caret and notified at pointer-down, before the
gesture was known to be a tap.

The runtime now waits for the release of a single Android finger tap.
Existing consecutive tap, long-press and accepted focused touch-drag
behavior is preserved. In particular, a focused Android finger drag
still follows with a collapsed selection, and an unfocused drag still
does not focus the field. A native control also discards a delayed
release when intervening focus changes invalidate the press. Three
focused Rust targets passed 27 tests. No extra upstream registration
credit was added for the authored comparison.
