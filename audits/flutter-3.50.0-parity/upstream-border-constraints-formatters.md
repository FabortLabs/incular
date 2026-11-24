# Upstream neutral border, constraint and text formatter contracts

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular `c647cb85`.
Fourteen named reference registrations passed. Eight have fully
comparable observable assertions (five match, three differ). The six
partial ports below are kept outside that count.

| Upstream file:line | Outcome | Evidence |
| --- | --- | --- |
| `painting/border_side_test.dart:79` | match | Default-alignment scaling, all five samples. |
| `painting/border_side_test.dart:109` | mismatch | Extrapolation across negative width returns a no-border side in Flutter, a zero-width solid side in Incular. Exact-zero hairline is a separate matching control. |
| `painting/border_side_test.dart:126` | match | Six stroke-alignment interpolation samples. |
| `painting/border_test.dart:27` | partial mismatch | `Border.symmetric` swaps left/right and top/bottom values. Flutter original also uses object identity; Rust uses `Copy` sides. Identity not compared. |
| `rendering/box_constraints_test.dart:17,60` | partial match | Finite bounds/equality and finite-to-finite interpolation match; nullable copy/lerp cases and the Dart hash assertion are not ported. |
| `rendering/box_constraints_test.dart:134` | partial mismatch | At t=-0.1, tight 10×10 to loose max100 normalizes to tight 11×11 in Flutter, Incular retains 10×10. The non-null height case matches; nullable cases are not ported. |
| `rendering/box_test.dart:755,776` | match | Constrained horizontal and vertical sizes are 100×200 and 200×100 in both; parity is the observable retained geometry, not render object type. |
| `services/text_formatter_test.dart:148,210` | mismatch | Deny gives `123\n456` with selection (1,5) vs (3,9). Allow gives `abc` with selection (2,3) vs (3,9). |
| `services/text_formatter_test.dart:304` | match | Non-BMP graphemes and equivalent end caret. |
| `services/text_formatter_test.dart:418` | partial match | ZWJ cluster/length/caret result matches through enforced editing; no static truncate API counterpart. |
| `services/text_formatter_test.dart:808` | partial mismatch | Pure deletion of entire composing text leaves the composing range 3..6 on another segment instead of clearing composing. The four string/pattern replacement cases had no Rust equivalent. |

Permanent named Rust targets: `flutter350_border_contracts`,
`flutter_box_constraints_parity`, `flutter350_services_text_formatter_parity`.
No full workspace or official upstream suite ran.

Two distinct custom border regressions were also confirmed and
committed with their pinned Flutter reference. Scaling a centered
or outside side in Flutter resets alignment to inside; Incular
preserves 0 or 1. Mixed None/Solid interpolation fades alpha in
Flutter (64/128/191 at .25/.5/.75), but stays 255 in Incular.
See `flutter350_border_side_edges` and `border_side_edges_test.dart`.
These are paint-data results, not an assertion of affected pixels.
The separate WGPU hairline fixture still covers actual raster output.
