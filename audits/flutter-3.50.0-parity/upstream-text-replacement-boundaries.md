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
