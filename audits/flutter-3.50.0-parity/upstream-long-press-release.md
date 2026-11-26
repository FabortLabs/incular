# Long-press release lifecycle

Pinned `gestures/long_press_test.dart:124,188` passed. A recognized
hold accepts a move from (10,10) to (100,200) without cancellation,
reports that exact position, and finishes only on release. The
separate stationary hold remains pending at 300 ms, starts after
the deadline, and produces one end. A duplicate release, a later
deadline and disposal add no callbacks. All 17 focused native direct
long-press and button cases passed.
