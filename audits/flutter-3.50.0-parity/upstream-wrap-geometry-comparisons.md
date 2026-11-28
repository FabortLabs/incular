# Upstream Wrap geometry and hit testing

Five exact named originals in pinned Flutter 3.50.0
`packages/flutter/test/widgets/wrap_test.dart` passed. The equivalent
assertions in `cargo test --locked -p incular-runtime --test flutter_wrap_geometry_parity`
also passed at Incular production `c647cb85`.

- 694, run spacing: 500×130 plus four vertical child offsets.
- 722, vertical and retained horizontal update: 150×510 then
  270×250 plus six child positions for each direction.
- 811, overlapping touch boundaries: exact four points; only (211,316)
  invokes the opaque GestureDetector.
- 1030, RTL space-between with one child: (700,0) in 800×600.
- 1045, zero area: a neutral runtime frame finishes successfully.

These cover observable geometry and hit behavior, not private render
objects or native window resize. They do not close the independent
fractional-fit C05 case. Logs under
`.tmp/flutter350-full-audit/wrap-widget-geometry-20261124-164936-092/`.

## Widget inspector properties

Pinned `widgets/basic_test.dart:1732` passed. Wrap exposes an
on-demand diagnostic snapshot with all eight original properties:
direction, alignment, spacing, run alignment, run spacing, cross-axis
alignment, text direction, and vertical direction. The comparison
uses the original three named children, 8.0/4.0 spacing, LTR and
vertical-up. It checks the INFO text and additionally verifies the
unset text direction and default down are the only filtered fields
in the default view. The snapshot uses typed enum and double
properties and does not mount or measure children. All 17 checks in
the focused Wrap constraints target passed.
