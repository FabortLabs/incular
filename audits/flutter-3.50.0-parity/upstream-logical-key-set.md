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
