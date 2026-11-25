# Render box constraints, offstage layout and retained reparenting

Pinned Flutter 3.50.0 `ab2912035e`. Four exact originals passed.

- `rendering/constraints_test.dart:13`: an 800×600 viewport contains a
  tight 200×200 FractionallySizedBox. Width factor 2 and height factor 0.5
  lay its child out at 400×100 without growing the box. All three sizes
  match through retained layout.
- `rendering/constraints_test.dart:34`: Incular's fallible constructor
  rejects all three original NaN combinations. This is partial: Flutter
  admits the values until validation and identifies each offending bound
  in a debug diagnostic. Rust prevents their construction instead.
- `rendering/offstage_test.dart:13`: the 10×10 preferred child lays out at
  800×10 under the enclosing tight width, but its painter never runs.
  Showing the same retained child then calls the painter once, without
  changing its geometry. This is partial: Dart exposes an explicit
  pre-layout `hasSize` flag; Incular's retained bounds read as zero then.
- `rendering/relayout_boundary_test.dart:9`: removing intermediate
  100×100 wrappers changes the global key's parent without replacing
  the element or render object. The custom layout counter stays at one,
  matching Flutter. A resize to 75×75 does trigger layout on that same
  render object.

Native targets: `flutter350_render_box_edges` and
`flutter350_constraints_normalization`. These are layout, validation,
and painter callback tests, not pixel comparisons.

The wider sizing check also turned up a stale ConstraintsTransformBox
test: it expected a clip for a child smaller than its parent. The updated
test covers fitting with clipping enabled, and overflow with clipping
both disabled and enabled. It preserves the prior Flutter-backed
[overflow-only clip rule](upstream-constraints-transform.md).
