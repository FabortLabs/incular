# Text selection identity and caret affinity

Pinned `services/text_input_test.dart:17` and `:28` now pair. The
signed UTF-16 Flutter selection type treats all invalid selections as
one equality/hash class. For valid selections, base/extent order and
the directional flag both matter. Affinity is compared only when the
selection is collapsed. The delta decoder uses this boundary type
without changing the retained editor's UTF-8 positions.

The editor previously changed affinity after publishing its selection.
Changing only the affinity at a bidi edge could leave a retained cursor
on the old side even though the controller already returned the new
affinity. A collapsed affinity change now invalidates the visual state
without changing the content revision, and observers see the actual new
side. An otherwise identical noncollapsed selection doesn't notify just
because its affinity differs.

In the authored Flutter probe and native paint test, the mixed Hebrew
and Latin string gives the two distinct caret edges at the same logical
offset (UTF-16 offset 7, native byte 11). The native test now verifies
that no offset change, focus transition, or manual caret reset is needed
to repaint. Both exact Flutter originals, one authored reference case,
and 45 focused native checks passed.

## Incular restoration regression

A new controller restoring the same mixed-direction caret now paints
it on the same visual edge as the saved controller. Before the fix,
the test reproduced a move from x=23.904001 to x=55.082 without a
logical offset change. Snapshot loading preserves affinity, the
directional flag, and whether the selection is set. Older snapshots
with only endpoints still restore to the previous downstream default.
An in-flight IME preedit is still cleared and undo history is rebaselined
on restore.

This is an Incular-only regression: Flutter's
`RestorableTextEditingController` stores only the text. It adds no
paired Flutter case. All 16 focused native checks passed.
