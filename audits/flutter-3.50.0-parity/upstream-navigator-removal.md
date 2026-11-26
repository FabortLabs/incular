# Imperative Navigator removal

The pinned `widgets/navigator_test.dart:1677` original ran. Removing
/B and then /A reports a removal with its actual predecessor, reveals
/A and then / immediately, and never reports a pop. The Rust observer
fixture checks the exact records and surviving route IDs. A separate
mounted Runtime test verifies the removed page stops painting on the
first frame even if it had a reversible transition.

Removing a route below the active route reports the removed route's
own surviving predecessor, not the active route, and does not change
the current page. This is distinct from the Dart tests that wait for
an optional typed route result; those are not claimed here.

## Push followed by a batch removal

The authored Flutter observer trace matches a push of /C, then removal
of /B and /A (both reporting the surviving / as their predecessor), then
one top change from /B to /C. No pop callback appears. Incular mirrors
that exact order and has a control with no surviving predecessor. This
does not change the existing incoming push and Hero transition path.
The authored fixture does not increase the upstream registration count.
