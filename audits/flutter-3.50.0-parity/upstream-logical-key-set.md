# Logical shortcut set identity

Pinned `widgets/shortcuts_test.dart` registrations :19, :72, :178
and :201 passed. The native key set retains one to four logical
members from either a sequence or an unordered set. Forward,
reverse and independently assembled inputs compare and hash
equally, including when used to retrieve or remove a map entry.
Repeated input keys are deduplicated.

Incular previously kept insertion order in the equality and hashing
representation, so the same chord could make two different map keys.
Construction now normalizes the key order before storing the set.
The four focused native tests passed. This covers set identity, not
the separate behavior of tracking held keys in a mounted Shortcuts
widget or the order in which the user presses a chord.

## Mounted chords and hardware state

The exact pinned registrations :112 and :238 passed. A mounted typed
shortcut invokes its action when either member of Control+C completes
the chord. Left and right Control both work. Holding Shift or A
prevents the match, key releases never invoke it, and every sequence
finishes with no keys held. In the activation query, actual focused
events match on down and repeat, not on either release.

The reverse-order Control+C sequence failed on the previous
CallbackShortcuts path because LogicalKeySet was converted to a
SingleActivator. Both the mounted typed scope and the callback
widget now match against the runtime's existing post-event hardware
state. The state is passed by reference and existing SingleActivator
modifier and lock rules remain independent.

The authored `shortcut_ordinary_chord_test.dart` probe checks A+B
in both orders, compares repeat handling with a SingleActivator,
and verifies that releasing an unrelated held C never triggers the
chord but makes a later repeat eligible. An additional Rust control
checks a chord with two ordinary keys and both physical Control
positions. Authored controls are not counted as original pairs.
The snapshot-free compatibility helper remains an event-only
approximation; native keyboard layout translation and registrar
registration are not covered here.
