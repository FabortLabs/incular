# Spell check services and spans

Both original registrations in pinned `services/spell_check_test.dart`
pass. The focused Rust tests compare the full printable value of a
suggestion span and a result containing checked text and ordered spans,
including exact delimiters, field names, and unquoted suggestions.
Empty and multiple-suggestion controls cover the same formatting path.
The surrounding service tests verify fallback, forwarding, saved results,
ordered merge and caret containment. These comparisons do not claim
native spell checker dictionaries or underline rendering.

## Retained widget span comparisons

All 13 registrations in pinned `widgets/spell_check_test.dart` ran
successfully, including every Android and iOS expansion. Incular's focused
`flutter350_spell_check_spans` test compares the public retained text span
tree: complete ordered text pieces, base style, red wavy misspelling style,
plain and composing underlines, and their platform-specific placement.
Cases include regions spanning or interrupting misspellings, and corrections
to stale results after insertions, deletions, or changes in whole-word matches.

Before the fix, five cases failed on Android: a missing composing range was
treated as a zero-length range at offset zero, adding empty children and a
spurious composing marker. The same tests now pass on both platform policies.
The existing focused spell-check tests also pass, including painted
misspellings. This does not verify native dictionaries or OS pixel output.
