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

## Remediation after the audit checkpoint

The observations above preserve the original baseline. The filtering fix
(`51fbd720`) maps the two selection endpoints and the two composing
endpoints in one UTF-8 traversal. The pinned authored probe and Rust agree
when filtering a supplementary scalar and a BMP scalar out of a mixed
reverse selection with active composition. Rust additionally checks snapping
malformed byte offsets and dropping a composing range that becomes empty.
This does not implement Dart regex/replacement-string APIs.

The length-limiter fix preserves the previous value for an overflowing edit
when already at the grapheme limit and the policy's guard applies. The
enforced policy uses a collapsed old selection; deferred uses absence of
old active composition. Both pinned authored cases passed, including
multibyte input at exactly the grapheme limit and the oversized replacement
of a noncollapsed old selection. The five named formatter targets pass 31
tests, and the retained editable widget's formatter-pipeline test passes.

Focused Rust targets: `flutter350_services_text_formatter_parity`,
`flutter350_upstream_formatter_filter_edges`, `flutter350_upstream_text_formatters`,
`flutter_text_formatters`, `length_formatter`, and
`editable_text_interaction::input_formatters_apply_in_order_to_user_edits_only`.

## Literal and regex patterns in retained edits

The pinned :29, :62, :618, :642, :674, :757 and :808 originals passed
by exact name. The public callback formatter forwards both original
values. Literal and regex filtering now replace each deny match, or
each nonempty gap between allow matches, without rescanning generated
replacement text. Deny and allow tests verify all original selection
and composing ranges, including an equal-length replacement that
preserves an interior point, unequal lengths that move the point to
the replacement end, overlapping literal candidates, and full removal
of a composing span. The retained editor uses the formatter for user
edits and leaves direct controller writes untouched.

:62 matches literal and regular-expression output and all explicit
selection offsets; it remains partial because Flutter also compares an
unselected -1 sentinel, while the Rust editing value has valid UTF-8
offsets. The paired authored Unicode probe verifies equal UTF-16
replacement lengths on both sides. An unrepresentable offset inside a
surrogate is snapped down to a scalar boundary in Incular. The regex
API uses Rust regex syntax rather than claiming support for every Dart
RegExp expression. The char-predicate deletion path remains separate.

The nine historical scalar filter and length-limiter mismatches
(:148, :164, :180, :195, :210, :225, :240, :257 and :445) also passed
against the permanent native targets and exact pinned reruns. The
deprecated aliases exercise the same filtering behavior; no obsolete
Rust alias was introduced.
