# Flutter 3.50 logical text boundaries

All twelve registrations in pinned `services/text_boundary_test.dart`
and their Rust counterparts pass. The ports verify every original
character/grapheme, word, line, paragraph, and document boundary.
Rust retains UTF-8 byte offsets; comparison fixtures translate the
reference's UTF-16 positions and inspect the actual returned locations.
The full two-endpoint query agrees with both independent endpoints.

Three supplemental Flutter probes cover non-ASCII CRLF, negative input
on an empty document, and real laid-out line lookup at a newline. Incular
previously placed the trailing character edge before the document at
the end of character 1, and could panic when stepping back from CRLF
into the middle of a UTF-8 scalar. It now returns the pinned endpoint
and steps by scalar boundaries. A line lookup treats a hard break at
the caret endpoint as the preceding line, without including the break
in the returned range or using the shaped glyph range's gap fallback.

Focused boundary, editor-state, and paragraph-document layout tests
passed. These are logical boundary queries, not a claim that every
platform keyboard binding already uses the same policy.

## Keyboard use of hard line boundaries

The authored `editor_hard_line_shortcuts_test.dart` and Runtime test
send Home, End, Shift+Home and Shift+End to a focused multiline editor
under both Linux and Windows shortcut policies. Before the repair, Home
at the first LF could jump to the last line (byte 12 rather than 0).
The editor now consults the logical boundary before falling back to
shaped-line lookup. End and Shift+End stop just before LF, including
after CR for CRLF as the pinned Flutter fixture actually does.
Both editors keep the drawn caret on the first line. The ordinary
single-line, wrapped, selectable-text and editor-scroll shortcut suites
still pass. These authored tests do not add upstream registrations.
