# BoxDecoration hit regions

Five pinned originals in `widgets/box_decoration_test.dart` pair
(:258, :293, :331, :372, :436). A border-only rectangular decoration
accepts hits across its interior; a circle uses the shortest side,
and a rounded rectangle excludes its cut corners. The native
fixtures click identical coordinates through a mounted
defer-to-child GestureDetector, not just a geometry helper.

The fractional-radius diagonal outside sample previously registered
a real click at (3.6025865, 3.6025865). Normalizing each corner axis
in f32 rounded the point back inside. The contains test now uses
f64 intermediates for the represented native geometry, matching
Flutter's double-precision equation without widening stored layout
or paint data. The test also checks adjacent f32 coordinates and
both edge tangencies. No clipping or border drawing was changed.

All five pinned originals and 22 focused native checks passed.
