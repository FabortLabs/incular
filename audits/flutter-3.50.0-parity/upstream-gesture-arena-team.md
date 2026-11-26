# Gesture arena teams

The two pinned `gestures/team_test.dart` registrations (13 and 64)
passed. Incular's `GestureArenaTeam` registers one managed-arena
contender for its horizontal and vertical drag recognizers. A real
outside tap wins the zero-displacement input sequence. A 30-pixel
vertical move instead starts only the vertical drag, or delivers
acceptance only to the configured passive captain. The native test
routes real recognizers and records their actual delivery callbacks,
including the empty trace during registration, close and press.

The team remains in competition while any member is undecided.
Default settlement without a captain accepts the first remaining
member; an extra native case rejects the first member before the
default boundary and verifies that the second wins. This is the
explicit managed-gesture path. The default widget detector still
supports its existing simultaneous-recognizer policy.
