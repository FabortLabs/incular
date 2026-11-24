# Focus, corner geometry, curves, text style, and previews

Pinned Flutter 3.50.0 `ab2912035e`, unchanged Incular `c647cb85`.
Durable regression and control paths are identified below.

## Focus

`focus_manager_test.dart:1752` expects a freshly mounted autofocus
target to take focus after the previous target detaches. It does in
Flutter; Incular's mounted replacement reports neither primary focus
nor a runtime target. :1801 first-of-three autofocus matches.
At :633, the public Rust host-equivalent comparison finds the unattached
node claiming primary focus early and an empty intervening frame
consuming its intent, so attaching it later does not adopt it. This is
partial because the explicit attached-but-not-parented Flutter phase
has no Rust host equivalent. :1233 blocked focus emits no per-node
notification and remains unfocused; labelled partial because the
original asserts a widget onFocusChange hook. Target:
`flutter_focus_upstream_parity`.

## Rounded corners and image fitting

The elliptical midpoint cases `border_radius_test.dart:165,423` match
all four per-corner values. Non-null quarter-lerp at :146/398 matches
12.5; nullable endpoints and missing arithmetic API are not claimed.
Custom `Radius.lerp(10×2,2×10,1.5)` holds (-2,14) on Flutter but
(0,14) on Rust. This is public value semantics, not a negative-radius
paint test. Targets `flutter350_corner_radius_contracts` and
`corner_radius_extrapolation_reference_test.dart`.

A separate real Windows wgpu paint check finds that a public 24×8
elliptical BorderRadius emits the same raster as a radius-24 circular
box. Flutter's ellipse is white, Incular is black, at (1,10), (62,10),
(1,53), (62,53); square and circle controls match.
`to_corner_radii` drops the Y axis. See
`flutter350_elliptical_border_radius_raster` and its pinned reference.
This is DPR 1 solid fill, not all radius clipping or all devices.

Official `box_fit_test.dart:9` is a partial mismatch. Public ordinary
Image paint gives full source and overflowing destination in two
FitWidth/FitHeight cases, versus Flutter's centered source crop
and exact bounded destination. All six positive size pairs match
through Incular's RTL-mirrored Image route; it uses the separate
BoxPainter helper. Negative and zero raw helper inputs are not
exercised through public decoded images. This is draw-command
geometry, not an additional GPU pixel run. Some old Incular tests
explicitly expect overflow; the audit does not resolve such policy
conflicts by changing production. Target `flutter_box_fit_paint_parity`.

## Curves and text style

`animation/curves_test.dart:28,278,291,511` match Threshold, Split,
five explicit-handle spline points, and CatmullRomCurve rejection
samples. :318 is partial: short point lists and invalid tension
reject on both, but non-finite points and endpoint handles return
success in Rust. The invalid Rust sampler was not run.
Target `flutter350_upstream_curve_geometry`.

`text_style_test.dart:480` expects component-wise shadow interpolation
at 0.5; Rust jumps to the right shadow list. :317 is partial: the
builder accepts NaN height (raw check), while Flutter debug rejects
it; the original paragraph override assertion has no direct
counterpart. Custom paired merge tests find explicit zero letter
spacing and empty fallback list fail to clear inherited values,
though omitted fields inherit on both. Target
`flutter350_text_style_contracts` and
`text_style_merge_defaults_reference_test.dart`.

## Widget previews

Three named original hermetic Flutter tool registrations passed
offline: manifest generation at :48, scaffold reuse vs regenerate at
:61, and pubspec-change regeneration at :91, all in
`widget_preview/preview_manifest/preview_manifest_test.dart`.
No corresponding Incular preview manifest/scaffold APIs exist;
the normal CLI build/run path and manual hot patch example are
not substitutes. No full Flutter tool suite, preview rendering,
or live DTD connection was run. Keep separate reference-only results.
