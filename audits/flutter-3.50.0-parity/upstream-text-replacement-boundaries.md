# Upstream text replacement boundaries

Seven named original registrations under `TextEditingValue replaced`
in pinned Flutter 3.50.0 passed. Incular's committed
`flutter350_text_replacement_boundaries` target matches two and
reproduces the other five failures, after the resulting text assertion
passes in every case.

| Site | Case | Selection outcome |
| --- | --- | --- |
| 44 | Forward selection deletion | Match: caret 5. |
| 66 | Insertion at collapsed caret | Match: caret 7. |
| 110 | Replace at start boundary | Flutter (14,5), Incular (7,7). |
| 126 | Replace at end boundary | Flutter (14,5), Incular (14,14). |
| 142 | Delete after selection | Flutter (13,5), Incular (13,13). |
| 158 | Delete at start boundary | Flutter (12,5), Incular (5,5). |
| 174 | Delete at end boundary | Flutter (12,5), Incular (12,12). |

All text and offsets in these fixtures are ASCII, so this is not a
UTF-8/UTF-16 reporting difference. The five red tests are variants of
the previously reproduced selection-collapse problem. This now
accounts for every registration in the original `replaced` group.
Other channel/device behaviors in the file remain open.

Rust: `cargo test --locked -p incular-text --test flutter350_text_replacement_boundaries`.
Originals ran in two exact-name batches; scratch
`upstream-runs/20261124-170458-425` and `20261124-170508-890`.

## Selection and composing repair

All ten original replacement cases now pass across the two focused Rust
targets. `TextEditingValue::replace` adjusts each end separately without
collapsing an unrelated selection or clearing composing. An insertion
at the caret still advances it, and deleting the selected range still
collapses the selection. The separate controller edit operation continues
to collapse a selection to the inserted text's end.

Three authored Flutter 3.50.0 and Rust Unicode controls also pass:
equal-length replacement across variable-width scalars, insertion at a
composing boundary, and a shorter UTF-16 replacement with the same byte
length. The Rust API retains its UTF-8 byte representation. An interior
UTF-16 position without a representable UTF-8 boundary snaps down.
All 24 Rust checks in the three focused targets passed. The historical
ledger classifications remain the findings before remediation.

## Composition range validity

Pinned `services/text_input_test.dart:1250` and its Rust port agree:
absent, inverted, or out-of-bounds ranges are invalid; an ordered range
ending exactly at the text boundary is valid. The authored comparison
also verifies that a collapsed range differs from no composition and
covers a supplementary-plane character. Incular's native offsets are
UTF-8 bytes and reject positions inside a UTF-8 scalar. Flutter's
negative sentinel is represented as absent in Rust. Checked text deltas
reuse this boundary predicate without changing their existing behavior.

## Pure replacement versus user editing

A pure replacement preserves the direction of a nonempty selection when
possible. Actual keyboard paste instead collapses the caret after the
inserted text, and Backspace collapses it at the earlier selection
edge. Both the pinned Flutter keyboard/clipboard probe and the Rust
runtime probe verify the distinction, including a reversed selection
across a supplementary scalar. The controller allocation fixtures now
use the same pre-edit selection when constructing the expected value
instead of conflating these two operations. These authored probes add
no original registration to the coverage ledger.

## Current text replacement disposition

The seven original historical mismatches at :78, :94, :110, :126,
:142, :158, and :174 were rerun by exact name on the pinned SDK.
The repaired `flutter350_services_text_state_parity` and
`flutter350_text_replacement_boundaries` tests passed without
exclusions. Their ledger rows now describe the current results.
