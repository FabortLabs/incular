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
