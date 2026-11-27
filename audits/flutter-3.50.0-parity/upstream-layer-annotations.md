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

### Region bounds are not subtree bounds

Five remaining region cases passed (487, 508, 529, 592, 624).
A region is transparent by default. Even a zero-size opaque region
still searches its children, and only a matching child may absorb the
background. The region offset changes its own bounds and subtracts
from its own returned position, not from any child's coordinates.
The original test-only child uses a shifted box for its search while
reporting the incoming point. The Rust fixture uses an ordinary
child clip to reproduce that distinction without a custom layer.

### Affine and leader placement

The five remaining Flutter registrations (281, 319, 418, 549, 571)
passed. Incular reproduces 281, 418, 549 and 571: a 2x/4x transform and
translation return the exact inverse-mapped point; a leader offset moves
its own child and reports the child's local point; and a transparent
child does not change whether its annotated parent absorbs the
background. Additional native tests verify that painting, the follower
link, scene bounds and cached clip culling all consume a changed leader
offset in the same coordinate system.

The perspective case at 319 is reference-only. The retained layer
transform here is affine; replacing it with a 2D approximation would
not preserve the reference's projective inside/outside tests.

### AnnotatedRegion widget lowering

Both pinned `widgets/annotated_region_test.dart` cases (10, 19) passed.
The Rust tests mount, lay out, and paint the actual `AnnotatedRegion`
widget with the sized child. They find its typed value in the published
compositor layer tree and reproduce the translated case: (10, 10)
misses, and (50, 50) hits value 1 at local (25, 25). Positions presented to
the Incular tree are logical pixels, equivalent to the DPR-adjusted
test points on Flutter's root layer.

### AnnotatedRegion search across retained layers

All 17 original registrations in `rendering/annotated_region_test.dart`
ran against the pinned SDK. Their full runtime test names begin with
`AnnotatedRegion<Object>`. Thirteen have equivalent native assertions in
`flutter350_annotated_region_search`: the three adjacent sized/offset,
rectangular, and rounded stripes; inverse composition of a 2.625 scale
with three local offsets; searching for a runtime type behind an unrelated
type; not clipping an integer child to an unrelated parent's 10×10 size;
and the nested front-to-back result `[3, 1, 2, 0]`. The tests assert both
first and all matches where Flutter does, with controls checking adjacent
stripes, trailing edges, and rounded corners.

Four cases stay partial. Flutter can search an annotated layer directly
while it remains mounted under another container (114, 325), and observe
a new transform immediately after mutating that layer (144, 368). The
native fixture verifies child-before-parent order when the queried layer
is the published root, and a singular transform hides the annotation
until the next flatten after resetting it. Incular's scene queries
intentionally use the published frame; these tests do not prove an
equivalent direct live subtree query.

### Reflected annotation boundaries

Both authored `reflected_annotation_edge_test.dart` cases passed against
the pinned reference. Before the fix, both native counterparts returned
no hit at (0, 5). A rectangular clip in a reflected space and a sized
region in the original space can share a leading edge that both contain.
The same holds for two clips surrounding an unsized region. Their world
AABBs touch without positive area, but the exact local half-open
containment tests accept the shared edge. The annotation collector now
preserves a touching world intersection and lets those local tests
decide. Checks immediately to either side and at the trailing corner
still return no hit. This adjusts annotation lookup, not painting,
clipping coverage, or renderer culling. Authored cases are not counted
as original registrations.
