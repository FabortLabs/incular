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

## iOS single-finger selection and toolbar

In pinned Flutter, tapping strictly inside a focused range keeps both
endpoints intact, toggles the toolbar without hiding the handles, and
does not send a selection callback. Outside the selection, a finger
tap over the interior of a word places the caret at the word's trailing
edge with upstream affinity. The pinned probe uses real EditableText
geometry and spies on the toolbar methods; the Rust test exercises the
actual context menu's open and close state. The old runtime collapsed
a selected range to the tapped character as soon as the finger went
down. The corrected path waits for a confirmed single tap and leaves
double taps, long presses, and the spelling path separate.

This is an authored paired comparison for iOS, not full credit for
the upstream multi-platform registrations in text_selection_test.dart.

### Original platform-variant tap registrations

The original `widgets/text_selection_test.dart:706` single tap and
`:730` tap on a previous selection both passed on all six Flutter
platform variants, in four serial batches of three exact names. The
native counterpart checks the same branch split with the real editor:
the ordinary tap reports `Tap` at the iOS word edge versus an exact
caret elsewhere, and a tap inside a selected range toggles the toolbar
on iOS versus moving the other platforms' caret and reporting `Tap`.
The runtime maps Fuchsia to the Android caret policy; these cases make
no assertion about an actual Fuchsia embedder. All five focused native
pointer cases passed. Count the two registrations, not twelve variants.

### Mobile Shift taps and precise iOS pointers

The pinned authored `mobile_editor_shift_tap_test.dart` uses physical
Shift and waits through a finger press before releasing Shift and then
the pointer. With a starting range of 2→5 and a tap at offset 1, Android
keeps base 2 while iOS anchors at the farther endpoint 5. Neither moves
the range on the press; each sends one `Tap` on release. The previous
runtime collapsed instead. The confirmed Android finger tap now uses
the modifiers recorded at press time.

The iOS mouse control keeps the range on press and cancellation, then
places a precise caret at offset 4 after a completed tap. The tap
route now waits for release on precise iOS devices, and a mouse drag
beyond tap slop cannot later turn back into a tap. Three pinned
authored cases and 40 native pointer, spelling, focus and handle tests
passed. No original registrations were added for these cases.
