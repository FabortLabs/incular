# Opacity repaint lifecycle

All five pinned registrations in `widgets/animated_opacity_repaint_test.dart`
and `widgets/opacity_repaint_test.dart` passed. Three have complete native
counterparts: the one-second fades from zero to 1 and 0.99, and the
static 0 → 0.1 → 1 progression. The mounted native custom painter runs
zero times while hidden, once on first visibility, and never again
during these opacity-only updates. The visible compositor layer ID
and source generation stay stable through the halfway frame, the
endpoint, and stopping the animation.

The dynamic and static zero-opacity disposal registrations stay partial.
Native cases show that their child stops contributing paint and never
repaints, but Incular deliberately keeps an empty retained opacity
marker with the same ID and generation zero. Flutter's test asserts
absence of an OpacityLayer at that endpoint, so these two get no
complete-match credit. The three targeted native cases passed.
