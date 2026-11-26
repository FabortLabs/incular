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
