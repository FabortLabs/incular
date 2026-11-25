# Spell check services and spans

Both original registrations in pinned `services/spell_check_test.dart`
pass. The focused Rust tests compare the full printable value of a
suggestion span and a result containing checked text and ordered spans,
including exact delimiters, field names, and unquoted suggestions.
Empty and multiple-suggestion controls cover the same formatting path.
The surrounding service tests verify fallback, forwarding, saved results,
ordered merge and caret containment. These comparisons do not claim
native spell checker dictionaries or underline rendering.
