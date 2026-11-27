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
