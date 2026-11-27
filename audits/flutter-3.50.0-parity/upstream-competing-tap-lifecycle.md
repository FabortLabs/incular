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
