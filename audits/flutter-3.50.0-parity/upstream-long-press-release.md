# Long-press release lifecycle

Pinned `gestures/long_press_test.dart:124,188` passed. A recognized
hold accepts a move from (10,10) to (100,200) without cancellation,
reports that exact position, and finishes only on release. The
separate stationary hold remains pending at 300 ms, starts after
the deadline, and produces one end. A duplicate release, a later
deadline and disposal add no callbacks. All 17 focused native direct
long-press and button cases passed.

## Tap down before a competing hold

Pinned original :142 passed. A mounted tap recognizer and a separate
mounted long-press recognizer share the same pointer. Only the hold
announces down on delivery. At 300 ms, the still-pending tap reports
tap-down; at one second, the hold starts. Disposal does not add any
callbacks. The native fixture pumps the real runtime deadlines; it
does not call either callback manually.

## Device filters, button changes, and disposal

Four more exact originals passed (:227, :254, :472, :497). Changing
the button before acceptance cancels once without starting later.
Changing it after acceptance preserves the current release and allows
the next contact to hold. A filtered mouse does not replace an active
touch, and a mouse-only recognizer ignores a touch then accepts the
same pointer ID from a mouse.

The pinned :163 pending-disposal case also exposed a real gap: a
routed hold logged only down before disposal, instead of down and
cancel. The standalone recognizer now cancels an unresolved routed
hold before clearing its state. It neither reports a cancellation
for an unrouted registration nor invents an end after acceptance.
That standalone comparison did not verify the surviving drag's microtask. All 34 focused native
long-press, button, and managed-arena controls passed.

## Surviving drag after a pending hold is disposed

The pinned original :163 now pairs. The real managed arena contains
a horizontal drag and the long-press recognizer. Down and the first
300 milliseconds leave both pending. Disposing the hold calls cancel
and withdraws it; the remaining drag cannot start on that stack.
Only the deferred arena drain starts it, once, at (10,10).

External arena awards and deadline timing are independent. An early
default award cannot start a hold before 500 ms. A deadline that
precedes an award cannot leak a start, move, or end. Release before
the award cancels and ignores a late result. The native suite includes
70 focused arena and gesture checks and ten mounted runtime controls.

## Button competition and late mask changes

Six exact originals (:569, :581, :596, :640, :701, :742) passed. In
the retained arena a primary hold does not delay a secondary
tap-down, but keeps a primary tap pending until early release.
A secondary hold cannot trigger primary callbacks; tertiary
cannot trigger either primary or secondary. Positive controls
confirm that configured button callbacks are live. A hold begun
with primary sends one primary end even after adding secondary and
then releasing primary in either of the original contact sequences.
Four standalone button tests and three actual runtime tests passed.
