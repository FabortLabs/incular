# Competing tap recognizers and deadline cancellation

Both pinned `gestures/tap_test.dart` cases (483 and 598) passed. There
are two registrations titled “No duplicate tap events”; :186 was
already credited and is not counted again. The native :483 fixture
adds two competing mounted recognizers and sends the two original
streams at (10, 10) and (30, 30) with their original release positions.
Each releases with down, up, and tap from the winner only, with no
extra callbacks after pumping timers.

The second fixture mounts a tap and a horizontal drag on the same
path. After a one-second clock advance the pending tap sends down
exactly once. Pointer cancel then sends cancel exactly once, never up
or tap, including after another timer pump. Both focused native tests
passed; no change to recognizer behavior was needed for these cases.

### Button-specific arena competition

Four pinned `gestures/drag_test.dart` registrations (1951, 1965, 1982,
2018) passed. A primary pan contender does not enter a secondary-button
arena: the sole secondary tap can report down immediately. With a
primary press the two contenders do enter, and the primary tap waits
until release. Sending the two original secondary drag sequences
through the retained pointer path invokes none of the primary pan
down/start/update/end/cancel callbacks and no primary tap. Four native
tests in the focused target passed.

### Held arenas, rejected taps, and overlapping deadlines

The five pinned `gestures/tap_test.dart` registrations at :375, :403,
:634, :665 and :1069 passed. The native arena verifies both decisions
after a held sweep: an accepting competitor suppresses the tap, and a
withdrawing competitor releases it exactly once on deferred resolution.
A second contact cannot arm or keep a deadline when the first contact
releases. The standalone test checks before the deferred award; the
mounted control checks after the binding has awarded the only contestant
between input samples. Neither produces another callback when time
advances 200 ms or the second contact releases.

Diagonal motion exceeding tap slop before the press deadline stays
silent, including after release and a timer pump. A horizontal drag
wins before the second contact arrives; even if that finger later lifts
while the drag is held, it does not restart the rejected tap. A fresh
contact after both releases still taps. Eight focused recognizer and
mounted tests and four adjacent mounted overlap tests passed. No runtime
change was needed for these cases.

### Primary tap contenders with different button masks

The three pinned `gestures/tap_test.dart` registrations at :895,
:911 and :927 passed. For the original primary touch, a mounted
secondary-only or tertiary-only ancestor does not delay the inner
tap down and receives no callback when that touch lifts. Two
primary listeners both remain pending at 99 ms, report down at
100 ms in hit-path order, and do not repeat at 500 ms. Three
focused and 12 adjacent runtime tests passed, with no production
change needed.
