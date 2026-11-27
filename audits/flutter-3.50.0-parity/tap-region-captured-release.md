# TapRegion release uses captured press membership

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
The gesture binding retains the hit path from the press and dispatches the
release through that path. `RenderTapRegionSurface.handleEvent` classifies
the current registrations against that cached path, not against a new
hit test at the release point. The callback still receives the real
release event and its current coordinates.

The authored Flutter and native fixture move in both directions across
the annotated region boundary, and across the enclosing surface boundary.
A press inside still gets an inside release even when the pointer finishes
outside the surface. A press outside that surface does not invent a release
for it. Cancellation sends no up event and clears the captured membership.
The native check originally sent the outside callback after crossing from
inside. The raw route now saves the relevant hit path with its contact and
prunes detached nodes, without changing ordinary hover or pointer routing.

This authored probe earns no new original-registration credit.
The pinned authored check and 18 focused native checks passed, including
the adjacent secondary-button, group, cancellation, and route cases.
