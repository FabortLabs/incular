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
