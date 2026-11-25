# AnimatedCrossFade focus, touch, ticker and semantics

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Five original registrations passed. In Rust the inactive child
cannot receive touches after the visible-child swap (:353), and
explicitly allowing bottom focus correctly moves focus to it
(:328). By default bottom focus is excluded (:304). Rust does
keep the previous Runtime focus target, but the bottom node
also claims primary focus after a direct request. Flutter leaves
that flag false. This is a public focus-state gap, not proof of
misrouted key events to the bottom child.

An outgoing child loses its semantic label while it still
ticks, then stops ticking once settled (:226). This partial
match measures actual controller progress and emitted labels,
rather than Flutter's exact internal ticker mute flag and
ExcludeSemantics descriptor. onEnd callback counting (:518)
is Flutter-only: the current Rust AnimatedCrossFade has no
public onEnd hook. No equivalent was simulated.
See `flutter350_cross_fade_focus_input_parity`.

## Current completion and clipping

The three pinned completion registrations (:424, :471, :518) and their
mounted counterparts agree: no callback on initial mount or halfway,
one on the completed forward/reverse transition, and one for each finished
cycle. Completion comes from the retained controller's status, not from
size or a value callback that could be skipped at an unchanged value.

Default hard-edge and explicit no-clip now propagate to both the outer
clip and the AnimatedSize clip (:557, :579), including a retained update.
This checks the installed clip policies rather than GPU snapshots.
The prior :304 hidden-focus failure did not reproduce on the current runtime:
an explicit request does not leave the inactive node falsely primary.
All six named Flutter originals and all eleven focused native tests passed.
