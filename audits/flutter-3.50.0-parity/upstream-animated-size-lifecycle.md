# AnimatedSize controller lifetime and moving children

Four focused pinned Flutter registrations passed. The native size controller
continues a width-only change to 150×100 at 100 ms and preserves the ongoing
100→150 animation and node identity when a globally keyed subtree moves
between siblings. Those two are paired.

The visible part of Flutter's unstable-child sequence also matches at the
same clock samples: hard-follow the changing child to 150 and 200, remain
at 200 while stable, then animate a sudden smaller child via 150 to 100.
Flutter's test also inspects internal phase names; that part stays partial.

Unmount previously left a controller live if an inspector or another
descriptor held a handle. It now explicitly disposes the controller, confirms
a second disposal is rejected, and leaves independently mounted copies
running. Dropping the entire tree releases it as well. Both unmount tests
failed before the fix. The original Dart controller diagnostic is partial:
the native curve is a value, not a second inspectable animation object,
and the duplicate-disposal wording is checked without the Dart object dump.

All 16 tests in the named native target passed.
