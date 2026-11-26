# Physical surfaces

A non-square PhysicalModel with a circular shape uses an oval
inscribed in its full bounds, not BoxDecoration's shortest-side
circle. At 100×40, paint at (2, 20) and (98, 20) was missing.
The physical model now paints the ellipse using the existing oval
path geometry, matching the authored pinned Flutter recording. Its
fill does not become a hit surface when its child declines the hit.
BoxDecoration's independent circle behavior is unchanged.

Two pinned `widgets/physical_model_test.dart` cases pair: the model
and custom shape both lay out and paint safely when constrained to
zero area (:119, :131). The two exact originals, two authored Flutter
checks, and ten focused native cases passed.
