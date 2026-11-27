# Tap and drag competition in a mounted arena

Five exact pinned `gestures/tap_and_drag_test.dart` cases paired:
:424, :453, :509, :543 and :649. The Incular tests mount two real
recognizer factories in a RawGestureDetector and route the original
moves through the retained arena. The winner and callback sequence
are asserted, not assigned by the test.

Two native defects caused the initial failures. Every erased factory
was keyed by the type of the Rc wrapper instead of the concrete
recognizer. A detector with multiple factories could therefore call
one recognizer twice for a sample and never mount its competitor.
After separating the keys, deferred acceptance still let a losing
pan process the same movement and lose its pending cancel. The raw
arena now resolves a candidate before forwarding that sample to the
next; the loser reports cancel before the winner starts.

The pinned controls cover a five-pixel mouse move beating tap slop,
an eager combined pan beating a pure pan, two vertical drags beating
their competing combination, and the non-eager combination yielding
to a pure pan. Six focused and four adjacent mounted raw arena tests
passed. Other tap-series timers and sole-member default wins are
independent cases.
