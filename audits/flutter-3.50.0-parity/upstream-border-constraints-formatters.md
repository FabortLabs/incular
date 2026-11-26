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

## Side copy and stroke paint

The pinned `border_side_test.dart:68,89` originals passed, and Rust covers
every asserted replacement and paint property. A solid side produces a
stroke with the configured color and width; an absent style produces a
transparent zero-width stroke description even with a configured width.
Both use source-over. Stroke alignment is applied to path geometry, not
encoded in either paint description. Border stroke paths use the same
conversion. Twelve focused Rust tests passed, including leading hairline
geometry and paint validation. Constructor validation remains a separate
contract from copying.

## Directional canvas painting

The exact `border_rtl_test.dart:933` registration passed. Incular's public
border painting API emits one green filled edge at the same nonzero
origin, with the same included and excluded samples in LTR and RTL.
A visible non-uniform directional border rejects a missing text
direction; uniform and invisible controls do not require one. Physical
borders and decorations share the same command construction, with
no extra display list on direct canvas calls. Nineteen focused Rust
checks passed. Inheriting a directional BoxDecoration at a widget boundary
and cross-type interpolation are separate work.

## Border outlines and rectangle edge containment

The pinned `border_rtl_test.dart:345` registration and Incular port cover
outer bounds and inner bounds after physical and logical stroke insets,
including samples exactly on all four edges. Before the fix the retained
path missed `(80,190)` on the lower rectangle edge. A single rectangular
contour now includes its boundary. The authored
`rectangle_path_boundary_test.dart` confirms both fill rules, clockwise
and reversed geometry, and samples 0.001 logical pixels outside. Paths
with holes and non-rectangular paths retain winding evaluation. Thirteen
focused Rust tests and the two authored Flutter cases passed. The
independent, previously recorded control-point bounds regression still
reproduced; this change does not alter curved path bounds.

## BoxBorder factories and width construction

Pinned `painting/border_test.dart:474` and the Rust port agree on
the four physical edges, uniform and symmetric construction, and
the directional factory. The Rust controls additionally resolve the
same start/end values in LTR and RTL. The value cases for `:18` and
`:27` also pass, including every side in a uniform border and the
two distinct sides in a symmetric border. Rust copies a small
value rather than returning identical Dart objects, so these
identity assertions are not paired.

Pinned `painting/border_side_test.dart:9` matches debug construction:
the default is a 1px solid black side, negative width is rejected,
and negative zero equals positive zero. The Rust typed builder now
applies the same debug validation as the direct constructor instead
of quietly turning a negative width into a visible hairline. An
additional Rust control rejects NaN. Release builds keep Incular's
existing safe clamp for invalid widths.

## Compound shape border interior and edge order

Pinned `painting/border_test.dart:465` matches for all four combinations
of specialized-interior support. A compound opts into the specialized
fill only if every layer supports it, then dispatches the paint exactly
once to the outer layer. Otherwise the decoration fills the outer path
without calling a specialized hook.

The authored Flutter/Rust geometry control uses a 40×30 rectangle and
2px/3px inside strokes. Combined insets are 5px, the outer path is
unchanged, and the innermost bounds are (5,5,30,20). A Rust recording
also verifies the two paint commands run outside to inside, and that
an outside directional start stroke moves from x=0 to x=38 under RTL.
This does not claim equivalence for the general Dart border-addition
operator or specialization for every built-in shape. The existing
separate superellipse precision assertion remains open.

## Outlined shapes default to no stroke

The pinned default checks for StadiumBorder, RoundedRectangleBorder,
RoundedSuperellipseBorder, OvalBorder, BeveledRectangleBorder and
ContinuousRectangleBorder all require an absent side. The four
rectangular shapes also use zero corner radii. Before the repair, the
Rust types inherited the standalone BorderSide default, causing seven
shapes (including CircleBorder) to draw a solid black 1px outline
without requesting one. LinearBorder held that side too, but did not
draw while all edges were absent.

A focused recording now verifies zero stroke commands and zero
dimension insets for all nine outlined shapes, including StarBorder,
which already had the correct default. Bare BorderSide still defaults
to a 1px solid stroke. CircleBorder's eccentricity property and the
optional-argument shape of the Dart LinearBorder factories are separate
unpaired portions of those originals.

## Native superellipse edge fitting

The prior Rust path used the legacy web cubic fit. A 100×100
rounded superellipse with radius 20 and an inside 2px stroke produced
an inner control bound near 1.935px instead of the reference 2px.
The pinned native engine uses a pair of tangent conics in each
octant before the circular cubic. Incular now follows that geometry
without changing render backends; the conic-to-cubic subdivision
does not allocate temporary vectors.

The authored pinned Flutter/Rust sample agrees on the outer and
inset bounds and all 42 near-edge containment samples, including the
top-center boundary itself. The WGPU stencil tessellator separately
keeps all mesh vertices within the inset bounds. This is path and mesh
evidence, not native GPU pixel equivalence or elliptical-corner parity.

Pinned `painting/rounded_superellipse_border_test.dart:170` matches
all six inside/center/outside inset cases for the superellipse and
circle. `:198` matches the midpoint from a 10px inside stroke to
a 20px outside stroke: 15px centered.

## Physical border addition

Pinned `painting/border_test.dart:61` matches. Disjoint sides combine
without duplication, equal-color compatible sides add their widths,
and zero-width absent sides are neutral. Incompatible sides keep
distinct paint layers instead of panicking or silently replacing a
color. Runtime geometry preserves Flutter's order: the right operand
is outside, with the left operand inset by the outside stroke.

The authored geometry and recording check uses a yellow 2px left
side and a magenta 3px left side. Flutter and Incular both put yellow
at x=1 and magenta at x=3 and start the inner path at x=5. The Rust
typed result composes with ShapeDecoration. Cross-type border
addition is not claimed by this physical Border + Border overload.

## Circle and oval paths

Pinned CircleBorder defaults and inner/outer path cases now match.
Eccentricity defaults to zero, preserves a centered circle at that
value, and reaches the full rectangular ellipse at one. The authored
comparison checks 0.5 on wide and tall rectangles, copying/scaling,
and clamped interpolation. Rust additionally records the 10px stroke
at its correctly inset centerline.

The pinned OvalBorder case now passes all scaling and interpolation
assertions and its inner/outer containment samples. Previously the
exact top-center vertex of the oval's inner path was excluded by
the winding test. The fallback includes existing extremal joins of a
single closed filled contour; it does not extend the shape to nearby
points or turn cancelling even-odd contours into a fill.

## Layout-bound arithmetic and view conversion

Pinned `rendering/box_constraints_test.dart:35` matches all four
finite arithmetic operations (including division truncated toward
zero and Euclidean remainder) on all four bounds. Rust rejects
arithmetic that would create invalid layout bounds; it does not
admit an uninvertible range just to store the Dart intermediate.

Pinned `:200` matches default-unbounded and finite host-view
conversion. The authored paired retained check actually sizes a
child to (2,3), then to (5,6), without replacing the child when
the view bounds change. The pinned `:17` copy-with case now pairs
all requested bounds and hash behavior. An unchanged copy hashes
the same, the changed copy does not, and equivalent signed zeros
share a hash. Three focused constraint tests passed. Numeric hash
values remain language-specific.

For `:95` and `:114`, the unbounded maximum and interpolation in
the other axis match, but the original uses a tight infinite
minimum. Rust deliberately excludes that value from layout
constraints. For `:121`, the new checked interpolation rejects
finite/infinite changes in either direction; the infallible
animation trait keeps its existing nearer-endpoint fallback.
The exact Dart assertion and its tight-infinite endpoints
therefore remain unpaired.

## Reverified negative extrapolation

The historical comparison above predates the repair. The exact pinned
:109 and both focused Rust border targets were re-run after the fix.
All five original extrapolation cases now match: negative widths are
absent, a genuine zero remains a solid hairline, and positive overshoot
retains its expanded width. The disposition is updated from mismatch
to match; this does not add a second registration.

## Symmetric borders with an omitted pair

The original constructor still passed at the pin, and the matching Rust
value checks are now independently backed by an authored paint comparison.
Flutter's raster and Incular's generated fill geometry agree at the left,
right, top, bottom, center, and exterior samples for a vertical-only and a
horizontal-only crimson border. The unspecified pair emits no visible
edge or hairline. This repairs the stale partial-mismatch disposition;
Dart's object-identity assertion remains inapplicable to the Rust value.

### Exact fallback error and all rounded corners

Pinned `painting/border_test.dart:235` is now fully paired at the
debug diagnostic boundary. The portable painter rejects the same
center/outside multi-color border and exposes one diagnostic with the
exact message and line breaks. Rust uses one panic payload, not a Dart
FlutterError node. The release paint fallback is unchanged.

Pinned `widgets/box_decoration_test.dart:487` and its mounted native
port both pass. The native control dispatches all nine actual tap
coordinates: the center and four interior corner samples fire; the
four square corner points outside the round arcs do not. All 13
focused runtime and widgets controls passed.

## Combining directional and physical box borders

Pinned `painting/border_rtl_test.dart:676` and `:710` passed by exact
name. The native cases verify that compatible sides add their widths,
inactive sides normalize to no border, and different colors remain
separate layers. All four forms of same-edge conflict remain two
layers. The mixed physical and logical side matrix tests both operand
orders and both text directions: physical sides stay put, logical
sides flip, and only unambiguous combinations merge. Empty results
actually emit no paint commands in either direction.

Direct border values now use the same stable display formatting as
compound borders. The compound description reflects operand order
while painting retains the independently verified outside-first order.
All six focused border-stack tests passed.

## Directional border hash behavior

Pinned `painting/border_rtl_test.dart:971` passed. Two independently
constructed two-pixel top borders compare and hash the same; moving
the identical side to the bottom changes both the hash and actual
paint coverage. Painting either direction produces exactly one edge
with an unchanged physical top or bottom. The physical Border value
has the same keyed-edge behavior. All 19 focused border tests passed;
the numeric hash itself is Rust-specific.
