# Opaque outlines and the shape fill

Flutter's `ShapeDecoration` insets a specialized solid-color fill by
half the stroke inset when its outline is both solid and fully opaque.
This keeps antialiasing coverage of the background from leaking past
the border. It does not inset a gradient-only fill, a translucent
outline, an absent outline, or the outline itself.

The authored reference records actual Flutter paint calls. For a
100-pixel circle and a 10-pixel stroke, fill radii are 45 for an inside
stroke, 47.5 for a centered stroke, and 50 for an outside stroke.
Opacity 254/255, no outline, and a gradient-only decoration also leave
the radius at 50. The baseline Incular solid fill was still 50 with
the opaque inside stroke. The fix uses the outlined shape's retained
stroke inset, leaving the stroke geometry, hit region, image clip
and shadow path unchanged.

The pinned authored probe and all 14 targeted Rust shape, hit and clip
tests passed. This verifies submitted paint geometry without replacing
wgpu. It does not claim the 75-shape overlap raster golden or add a
registered upstream case.
