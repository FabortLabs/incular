# Directional keyboard focus paths and reversal

Flutter 3.50.0, `ab2912035e160d9cad6306df307576da6128ebdb`:
`packages/flutter/test/widgets/focus_traversal_test.dart`.

Four original registrations passed in the pinned runner: moving around the
four corners, selecting the first target from each direction, retracing
directions on a pyramid of controls, and restarting directional history
after an explicit focus request. The native mounted tests use Incular's
arrow-key route instead of invoking the Dart policy from a FocusScope.
They check the same targets, per-control focus callbacks, scope membership,
and initial geometry selection. Incular reselected a geometric neighbor
instead of retracing the pyramid path before the fix.

The runtime now remembers actually visited stops on an uninterrupted
directional run. Changing axes, an explicit focus change, or an invalid
predecessor restarts the search. Reversing within the same scope and
scrollable uses the prior stop if its focus node is still mounted and
focusable. Wrapping is not appended as a forward step.

A separate authored probe confirms that removing the prior stop, or
replacing its focus node in place, falls back to a fresh geometry search
rather than requesting focus on the old or replacement node. Both Dart
variants passed. All 20 native tests passed across
`flutter350_directional_focus_paths`, `flutter350_focus_key_routing`, and
`focus_directional`. The two authored variants add no original credit.

Still open: Dart's direct scope and policy APIs and the configurable
directional scope edge policies. The native tests do not claim these.
