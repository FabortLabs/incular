# Text formatter bounds and live composition

All sixteen selected original Flutter registrations passed on pinned 3.50.0, as did one authored composition test. Incular's focused binary passes six tests and intentionally fails four.

- :180/:195: removing a newline yields matching text but leaves Rust selection (3,9) instead of (3,8).
- :240/:257: digits-only yields the same six digits, but Rust leaves the selection at (3,9), even though the string is only six bytes, instead of remapping to (1,4). The original deprecated-name rows actually run the same canonical Flutter helpers.
- :274/:289/:379: normal truncation clamps both directional selection endpoints at the limit, preserves valid start coordinates and preserves the empty case.
- :328 partial match: a unicorn followed by a space and the Latin XY example truncate to a single grapheme. Dart's caret inside the UTF-16 surrogate pair has no valid UTF-8 offset analogue; the Rust fixture checks a valid caret at the emoji boundary.
- :437/:453: under-limit edit and truncation when old text was already over the limit match.
- :445: at the limit, Flutter preserves the old ten a's instead of dropping existing text after a middle insertion. Incular truncates the proposed value to aaaaabbbbb. This reproduces the older C30 finding using the original specimen.
- :465 partial: Incular's portable default enum is enforced, matching the Windows-native reference value; there is no platform-dispatch getter or web default check. :475 remains Flutter-only due to that missing macOS policy entry point.
- :22/:29/:62 remain reference-only. Rust has no equivalent to const object identity, the built-in closure wrapper, or exact pattern/string replacement through its single-character filter.

Authored composition control: when old text is below the limit, enforced, deferred, and unlimited policies match across active/committed updates. At the limit, proposing a middle insert into non-composing 'ab' preserves 'ab' and the original caret in Flutter under both enforced and deferred policy. Rust produces 'aX' for enforced and 'aXb' for deferred, both carrying the new composing range. Unlimited is a passing positive control.

Run Flutter with run-reference-probe.ps1 -Test formatter_active_composition_test.dart and Rust with cargo test --locked -p incular-widgets --test flutter350_upstream_text_formatters -- --nocapture.

## Digits controls and filtering at selection boundaries

All fifteen remaining original formatter registrations passed, with the duplicated cursor title matched twice. The focused Rust follow-up passes four tests and intentionally fails two.

- :164/:225 repeat the exact canonical deny/allow behavior already covered at :148/:210, with the same selection-remapping mismatches and no obsolete Rust alias required.
- :397/:403 partial match: the formatter's ordinary update path truncates ordinary digits and a leading non-BMP emoji at the correct ten-grapheme boundary and leaves a valid caret. This checks representative outcomes, not a public static truncate helper.
- :483/:528/:544: the same digit text survives the disallowed '@' append. These originals compare text; their titles do not establish old editing-value identity.
- :499/:560/:589: when the proposed text needs no filtering, end caret 6 and middle caret 4 are unchanged on both sides.
- :661: the selected ASCII string filters to abca@bcabc as expected. This is a predicate analogue for exactly the tested input characters, not regex empty-match semantics across other inputs.
- :674 partial mismatch: the exact removal subcase 'AAABBBCCC', backward selection (6,3), produces AAACCC but fails to remap the selection to (3,3). Rust leaves (6,3). The other multi-character replacement-string subcases are not representable.
- :618/:642/:757 reference-only: Rust's single-character filters don't accept the original multi-character pattern plus replacement-string semantics.

Additional paired control (not an original test registration): when digitsOnly sees a proposal 'X' with no matches, both frameworks produce empty text, not the old '42'. Flutter clamps the caret to 0; Rust leaves it at 1 in the empty value. The authored test verifies this independently of a downstream text widget.

Run Flutter with run-reference-probe.ps1 -Test formatter_no_match_test.dart and Rust with cargo test --locked -p incular-widgets --test flutter350_upstream_formatter_filter_edges -- --nocapture.
