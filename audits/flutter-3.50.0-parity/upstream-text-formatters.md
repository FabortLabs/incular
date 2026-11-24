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
