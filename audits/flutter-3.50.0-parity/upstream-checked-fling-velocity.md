# Checked scroll-view fling velocity tracking

Three exact pinned `gestures/velocity_tracker_test.dart` registrations
(82, 96, 114) passed. Native checked iOS tracking requires at least one
sample, reports an earlier timestamp after a one-hour sample, and
continues estimating after each of five inputs one second apart.
It reports typed errors rather than Dart assertions. Rejected samples
do not enter the history, equal timestamps remain legal, and the iOS
weighted sequence settles to −1 logical pixel/s. Empty macOS fling
history also errors; the default least-squares empty estimate is zero.
The internal OS gesture path retains its previous tolerant behavior.
Three focused error fixtures and four existing tracker/fling tests passed.
