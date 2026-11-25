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

## Border-side repair and merge

`BorderSide.scale` resets stroke alignment to inside. Every intermediate
None/Solid interpolation uses a solid stroke with interpolated alpha;
the exact endpoints still return the original styles. Same-alignment
None/None interpolation retains the correct layout inset without painting.
Negative extrapolation returns an absent side; exact zero remains a
solid hairline. The `Border.symmetric` value mapping now puts vertical
edges on left and right. The upstream identity check remains Rust-inapplicable.

The original merge registration at line 14 of `border_side_test.dart`
now pairs across all 18 combinations, including each rejected color/style
mismatch and neutral zero-width absent sides. An extra Rust control checks
that a successful merge keeps the farther outside alignment.

All 14 focused Rust border/decoration checks and three authored Flutter
value checks pass. Five retained Flutter and Windows wgpu frames match
at all seven horizontal samples, including an extrapolated negative-width
side with neither a reserved inset nor an unwanted hairline. Translucent
interpolation is checked as paint data here, not as a raster color comparison.

## Constraint interpolation repair

Finite pairs now normalize their interpolated bounds instead of snapping
to an endpoint when the animation overshoots. For the tight 10 to loose
100 case at t = −0.1, both engines return tight 11. The nullable API
interpolates from or to zero and returns absent for two absent inputs.
All original assertions at `box_constraints_test.dart` lines 60 and 134
now pair, and the six focused Rust tests pass.

Rust still limits minimums to finite values and uses a safe nearer
endpoint for incompatible bounded/unbounded maxima rather than asserting.
The separate Flutter cases using infinite minimums stay open.

## Constraint normalization and empty sizes

Pinned Flutter originals at `rendering/box_constraints_test.dart:186,210`
and their Incular ports in `flutter350_constraints_normalization.rs` pass.
An inverted width (3..2) normalizes to tight width 3 without changing the
11..18 height. An empty size of 15×0 under 10..20 bounds on both axes
becomes 15×10, without dividing by the zero axis. Rust also checks the
transposed example and rejects NaN and positive-infinite minimums instead
of admitting invalid layout values. All 11 focused Rust tests and both
exact Flutter registrations passed on Linux; no raster output is involved.

## Box border defaults and stroke insets

Four exact Flutter painting registrations at
`border_test.dart:94,108,120,256` passed with public Incular equivalents.
They cover scale to zero, width/color/style/alignment uniformity, edge
order, and the inset of inside, centered, outside, and mixed strokes in
both physical and directional borders. Before the fix Incular reported
10 for a centered 10-wide side instead of 5, and directional dimensions
had no direct API. Decorations already used stroke insets internally;
the public physical query now uses that same calculation.

The separate authored `directional_border_defaults_test.dart` showed that
Flutter omits all unconfigured directional strokes. Incular previously
defaulted each side to a one-logical-pixel solid stroke, reserving and
painting a line even when no side was requested. After the fix, omitted
sides produce no paint command or inset. An explicitly requested
zero-width stroke still produces one hairline on the leading edge in
each text direction. Both the Flutter canvas and Incular display list
assert the zero width and edge coordinates; no new GPU raster comparison
was run. Sixteen focused Rust checks passed.

## Non-uniform border paint validation

Both pinned originals at `border_test.dart:235,335` passed. The
previously failing Rust paint tests now reject unsupported combinations
in debug mode: non-inside alignment on the plain rectangular fallback,
rounded multicolor borders, and rounded non-uniform hairlines. Valid
rounded or circular, single-color borders still paint despite different
widths or alignments; invisible edges and square hairlines remain valid.
The Rust paint-boundary port exercises every accepted and rejected case
at :335 and tests directional resolution on both sides. The specific
error at :235 is partial because Flutter's diagnostic-node type and
line wrapping are not reproduced. Thirteen focused Rust checks passed;
release fallback and the renderer are unchanged.

## Physical and directional border values

Five exact Flutter registrations passed: `border_test.dart:37` and
`border_rtl_test.dart:567,606,618,852`. Incular can merge corresponding
sides of either border representation, preserving a solid side when paired
with an omitted side and rejecting incompatible strokes. Directional
dimensions keep start/top/end/bottom order, scaling preserves the logical
edges and color, and uniformity compares width, color, style and alignment.
Thirteen focused Rust checks passed. Mixed-representation composition and
nullable interpolation are separate contracts.

## Nullable border interpolation

The pinned `border_test.dart:195` and `border_rtl_test.dart:866` passed.
Each public Rust port checks the same 24 cases at six factors: absent
endpoints stay absent only when both are missing; a single present end
scales in or out; two present ends interpolate their own edge values.
Scaling to zero produces an absent stroke, while a solid pair that
extrapolates to exactly zero still yields a hairline. Logical sides
remain independent of direction. Fifteen focused Rust tests passed.
This does not cover transitions between physical and directional types.
