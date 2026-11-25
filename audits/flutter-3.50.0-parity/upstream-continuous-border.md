# Painting: continuous rectangle border

The Flutter 3.50.0 border can resolve directional and mixed corner
geometry, and can describe the inner path even when stroke width makes
its edges cross. Incular previously accepted only physical radii and
could panic on that input. The path now keeps signed edges until it is
built. This does not change the outer paint stroke or the exact pinned
axis convention (top-left x goes down vertically and y horizontally).

Five exact Flutter registrations pair copy/equality/hash relations,
scale and interpolation, and zero, circular and directional inner and
outer path sampling. Two authored controls pair the inner path bounds
of a narrow box with a thick stroke and elliptical corner axes.
Seven focused native tests and 25 adjacent paint and outline tests
passed. The Flutter raster goldens were not compared in this slice.
