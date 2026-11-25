# Attributed accessibility strings and service boundaries

Pinned `semantics_test.dart:901` and the corresponding Rust comparison agree
on the complete concatenated string, both ordered attributes, their shifted
ranges, their kinds, and the diagnostic string. The authored UTF-16 fixture
checks that the right operand begins at unit 3 after an emoji and an ASCII
letter, not at byte 5; the right source span remains unchanged.
Incular exposes a fallible constructor for invalid ranges.

The pinned `:70` SemanticsConfiguration plain/attributed string replacement
case also passed on Flutter. The public Rust value is implemented, but its
configuration, merged-tree and adapter wiring remain separate work; do not
count the original as paired yet. The Flutter speech-span publication test
`semantics_update_test.dart:95` is likewise still reference-only.

The pinned Semantic announcement and sendEvent error originals passed. They
verify directed announcement data and failure reporting on Flutter's
accessibility channel. Incular has live regions, but no equivalent transient
announcement service or channel error path and does not claim those registrations
as paired. No native screen reader was invoked.
