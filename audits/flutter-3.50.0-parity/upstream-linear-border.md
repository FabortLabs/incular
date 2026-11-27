# Painting: LinearBorder edges

The pinned Flutter 3.50.0 edge fixtures compare all four paint regions
on a 100 × 100 rect, using the same translucent green. Both LTR and RTL
are tested, including all four boundary corners and the center on each
drawn edge, and the same five excluded points on the opposite edge.
Incular had the right extents but returned false at corners: its
filled contour was not explicitly closed. Closing the contour matches
Flutter's fill-boundary semantics without closing the zero-width
stroke or introducing an extra paint command.

Seven upstream registrations are paired: `:23` and `:28` defaults,
`:66` copy/equality/equal hashes, and `:117`, `:131`, `:145`, `:154`
drawn edges. Raw hash numbers are runtime-specific. `:73` and `:79`
are partial: both-null and equal-value cases pass, but Rust's Copy
types cannot expose the Dart object-identity assertion.

Three authored Flutter controls confirm scaling clears selected edges,
a selected solid zero-width side is a hairline, and invalid relative
edge sizes assert in debug. Eight paired/extra Rust controls and 30
neighbor paint and outline checks passed. These path-containment tests
are geometry/command comparisons, not raster-image comparisons.

## Diagnostic string

The pinned original `LinearBorderEdge, LinearBorder toString()` passed on
Linux. The formatter distinguishes omitted edges from explicitly configured
zero-length edges, omits default edge fields, and lists start, end, top and
bottom in Flutter's order. It uses the same BorderSide diagnostic string as
other outlines. All nine focused LinearBorder tests passed, including
existing RTL geometry, zero-width hairline and no-ink checks. Four nearby
Stadium morph tests remained green. The original skips browser builds.
