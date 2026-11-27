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

## Retained compact BoxDecoration

The ordinary uniform rounded border used a compact retained path that
bypassed the full decoration painter's background adjustment. In the
pinned paint-call probe, a 250 by 250 box with corner radius 75 and a
50-pixel opaque inside border starts its fill at (25,25). At opacity
254/255 it starts at (0,0). That distinction applies to both color
and gradient fills. The border still uses the original outer box and
stroke width, and the shader keeps its original coordinates.

The actual retained Rust command list now matches the paint geometry
when the same decoration toggles both ways, without relayout or
replacing the render object. It still emits one analytic fill and one
analytic border; the low-level brush-only constructor stays unchanged.
The pinned authored probe and seventeen focused Rust controls passed.
This does not claim the full raster golden registration.
