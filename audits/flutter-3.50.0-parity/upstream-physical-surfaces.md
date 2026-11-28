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

## Changing a physical clip

Pinned :19 and :39 now pair. Changing only the clip mode keeps the
same mounted physical proxy, child element, and child render object
while changing the actual clip commands. A focused native runtime
test also switches antialiasing on and off while typing into the
same nested editor. The old code remounted that editor because
Clip.none removed its wrapper entirely.

PhysicalShape's custom path governs hits independently of whether
painting is clipped. PhysicalModel's implicit oval can clip paint
without limiting hits in the child's rectangular bounds. Both hit
cases also passed the authored pinned Flutter reference. Fourteen
focused native tests passed. This does not compare the two renderers'
shadow algorithms or edge pixels in a save-layer composition.

## Circle tangent and property propagation

The pinned `widgets/basic_test.dart:118,134` cases passed. A real
800×600 native hit test uses the same shortest-side circle centered
at (400,300), blue fill, green shadow, and elevation 2. Only
(100,300) hits the child. The other three probes, (99,300),
(100,299), and (100,301), miss even with the default Clip.none.

The separate native paint check confirms one blue circle with bounds
(100,0)–(700,600), the green shadow, and the change in the shadow
effect when elevation changes from 2 to 4 and then 0. :118 remains
partial: Incular supplies precomputed path geometry instead of
retaining a ShapeBorderClipper as the render-object property, and
uses a non-painting child to supply layout. No shadow pixel
equivalence is claimed. All ten focused native tests passed.
