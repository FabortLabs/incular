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
