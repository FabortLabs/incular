# Sliver axis and geometry protocol values

All six pinned `rendering/slivers_helpers_test.dart` registrations
ran. The native cases pair all eight forward/reverse physical
direction mappings, default zero geometry, and both failures for
layout or maximum-paint extent violating the actual painted extent.
The typed validation error does not mutate those inputs.
The authored controls also exercise the pinned precision tolerance
and a supplied zero scroll correction.

Constraint copying, equality, hashing and normalized growth are
verified. These two rows are partial because the native protocol
does not have independent cross-axis and user-scroll-direction
fields like the Dart descriptor. The shared normalized-growth method
also drives the physical direction used by sliver paint bounds.
