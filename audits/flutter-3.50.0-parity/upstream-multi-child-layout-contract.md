# Multi-child layout queries, invalid child counts, and live relayout

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Nine exact original Flutter registrations passed. Initial loose
800×600 constraints, delegate box 200×300, and child sizes
150×100/100×200 match (:164). Changing preferred size also
matches and zero-area parent clamps the result to zero (:229,
:435). Nested layout (:205) compares independent repeated slot
IDs with separate Rust delegate descriptors, rather than Dart's
same delegate object; partial.

The replacement must decide whether relayout is needed (:182).
As in the single-child case, Rust instead asks the old receiver
((1,2),(1,3)). The replacement never sees either request and
the true request does not relayout.

Flutter rejects laying a child out twice or omitting it entirely
(:320, :388, :410), but Rust accepts both. The duplicate layout
can overwrite 150×100 with 100×90. An omitted child quietly
receives zero size, for both the one- and two-child omissions;
these are one shared fallback. These tests verify rejection or
accepted layout, not exact Dart diagnostic text.

The listener-driven case (:257) is partial. A shared retained
size changes from 100×200 to 150×240, but without replacing
the descriptor Rust keeps the stale size. A replacement with
new state works. Rust has no equivalent public relayout Listenable.
See `flutter350_multi_child_layout_parity`.

All four remaining original invalid-input registrations also pass
on pinned Flutter. Trying to lay out or position nonexistent ID 1
(:308, :351) raises the typed Rust missing-ID diagnostic. Negative
maximum constraints (:332) are rejected at Rust construction even
before layoutChild; Flutter reports them at the layout call. These
three are partial matches, not exact Dart diagnostic or timing
equivalence. An unlabelled child (:363) is instead accepted and
silently laid out as 0×0, unlike Flutter's rejection. This reaches
a related zero-size fallback but begins with missing identity,
not a valid identified child omitted by the delegate.

## Current replacement delegate direction

The replacement instance, not the previous one, receives the relayout
question. A no-relayout result preserves the measured geometry but still
installs the new instance, so the next comparison sees that replacement.
The exact single-child :121 and multi-child :182 originals both passed
on the pinned SDK. The two focused native counterparts and eight adjacent
custom-layout checks also passed.
