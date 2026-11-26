# Rendering layer annotation lookup

### Frontmost and opaque results

Five pinned `rendering/layer_annotations_test.dart` registrations (12, 32,
46, 59, 72) passed. Native retained-layer tests verify all three typed
entries arrive in reverse paint order with their local positions, the first
result comes from the front, and an absent type yields an empty list or
`None`. An opaque match stops the lower siblings. The native controls
also verify that opaque children leave ancestor annotations eligible, do not
stop another type, and that the annotation's own offset does not apply to
its children. Sized regions exclude their right and bottom edges.

The fix keeps lookup on the published scene, not mutable renderer inputs.
One pass records entries in hit order with a direct jump to each enclosing
annotation. It does not scan every retained layer when querying a point.
Ancestor clipping still uses conservative world bounds in this batch; exact
curved clip hit tests are tracked separately.

### Exact clips and inverse transforms

Five pinned cases passed for `OffsetLayer` (137), `ClipRectLayer` (163),
`ClipRRectLayer` (196), `ClipPathLayer` (236), and a singular
`TransformLayer` (399). The Rust fixture tests the same points and local
positions using real retained layers. To isolate the requested clip, it
uses a covering native annotated layer in place of Flutter's test-only
annotation override; the layer's own offset is independently covered above.

An annotation no longer intercepts a point through a rounded corner or
outside a polygon while inside its bounding box. The query maps into
each ancestor clip's local space and checks the actual shape. AABB
geometry remains a quick rejection and rendering culling remains unchanged.
The triangle's exact diagonal is included; an authored control verifies
that a retraced line does not become a filled edge. Further native
controls cover rounded clips under rotation and half-open rectangular
edges. Singular transforms do not participate, including for unsized
annotations. Perspective transforms are not covered by this batch.

### Absorption through nested layers

Five pinned registrations (91, 105, 120, 444, 466) and the corresponding
retained-layer tests passed. A transparent container forwards its child's
opaque match across the container's own siblings; a transparent match
or an empty sized child leaves the background eligible. An annotated
parent adds its own match after its child, even when that parent is
opaque. Its opacity then stops the background, not the already visited
child. These cases assert the complete result order and local positions.

### Status-bar consumer

The authored `status_bar_rounded_annotation_test.dart` reference passed
on Flutter's Android, iOS, and Fuchsia widget-test variants. Both widget
trees place a green annotated foreground at the status-bar sample point
over blue. At local (1, 1) the rounded corner clips it and the style is
blue; at (6, 6) it is inside and the style is green. Incular's runtime
asserts the actual style drained for the host after its frame, rather
than only exercising an isolated layer query. No physical status-bar
rendering was claimed and this authored case does not change the tally.
