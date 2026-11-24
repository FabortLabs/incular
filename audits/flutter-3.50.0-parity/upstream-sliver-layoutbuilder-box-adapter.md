# SliverLayoutBuilder viewport geometry and hit routing

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All five original registrations pass on Flutter. Rust's
no-MediaQuery-read handler stays at one call (:415).
Transformed positions (:538) match before and after the
original 100-pixel programmatic jump, including with
exactly viewport-height content. All padded, scrolled,
outside-viewport and no-content click probes (:587)
match the original routing. The Rust objects are public
box-adapter equivalents and PointerWithMetadata drives
the actual Runtime input path.

Consecutive builders receive (800,600) then (800,594)
and produce a 6-high and 20-high box (:53). Rust uses
boxes of the original padding extents, not nested
RenderSliver geometry objects. Stateful descendant
growth (:151) matches measured widths 800, heights
20/200/900 and adjusted max scroll range 0/0/300.
The original raw SliverGeometry paint fields are
not exposed by this Rust test. The latter two are partial.
See `flutter350_sliver_layoutbuilder_parity`.
