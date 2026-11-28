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

## Retained speech span metadata

Incular's Semantics now accepts attributed labels, values, and hints.
Plain and attributed setters replace the same field, and a plain replacement
clears only that field's attributes. Runtime tests check exact text, span
kind and ranges in the mobile projection, then verify a stable node's
updated strings no longer carry old spans.

The paired authored merge probe confirms offsets after a supplementary
character, newline, and Unicode direction mark: 4..6 for the opposing
contribution and 3..5 after its direction aligns. Both histories keep the
same merged node. These cases do not increment upstream coverage.
`:70` and `:95` are now partial-match: the observable property and portable
mobile update data match, but Flutter's exact Dart widget diagnostics and
OS-level pronunciation are not equivalent. Desktop keeps the resulting
plain label and base direction; pronunciation spans are not natively mapped.

## Ordinary nested annotations

Pinned widget originals `widgets/basic_test.dart:616,701` passed.
Incular initially emitted multiple nodes for these lookalike nested
annotations. The runtime now merges compatible labels and hints
without an explicit MergeSemantics widget, matching the originals:
`label\nlabel` has spell-out ranges 0..5 and 6..11;
`hint\nhint` has ranges 1..2 and 6..7. A plain middle label
contributes to the UTF-16 offset without creating a pronunciation
span; the final span in `label1\nlabel2\nlabel3` is 15..17.

A retained-update check changes the plain label to include a
supplementary character and verifies the next range immediately moves.
Adding an explicit container keeps the final label on its own node
with its original local range. All 17 tests in the attributable
text and semantics strings/locale targets passed. This verifies
portable nodes and metadata, not spoken output.
