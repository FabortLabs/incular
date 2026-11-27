# Resolving border radii without text direction

Both pinned `painting/border_radius_test.dart` registrations (348 and
363) passed. A directional radius, even one with identical corners,
needs a direction. Mixed values produced by physical-plus-directional
or directional-minus-physical arithmetic also cannot resolve without
it. The native behavior is a typed `BorderRadiusResolutionError` with
the same diagnostic signals, rather than throwing a Dart exception
or silently guessing LTR. A physical value passes through unchanged
with no direction. The same corner inputs also resolve to exact,
different physical corner results for LTR and RTL.

This supplies an optional `try_resolve` path alongside the existing
explicit resolution API. It does not change an already-resolved
renderer path or claim a change in ambient widget defaults.
