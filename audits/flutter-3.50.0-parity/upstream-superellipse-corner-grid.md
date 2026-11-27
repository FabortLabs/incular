# Rounded superellipse outlines and hit testing

The pinned-engine probe covers square, circular, elliptical, oversized uniform,
oversized asymmetric, partly sharp, and 100,000×500 shapes. Production painted
and inset paths agree with all 6,776 Flutter containment samples across the
four corners. The test also compares 3,388 direct shape hit tests.

Before the fix, 120 hit samples differed despite the painted paths matching.
Flutter analytically checks raw radii limited per quadrant, while paint scales
all radii together. A retained decoration can therefore paint at a local
point such as (50.35, 1.35) without accepting a tap there when the requested
radius is 600 in a 120×300 box. Both the Flutter widget and the Incular
decoration now repeat that behavior, and the center remains hittable.
The native check no longer allocates a path to test the shape.

The renderer remains WGPU. The two Flutter probes, two native comparisons,
and four rendering path controls passed. These are authored geometry and
hit-testing checks, not full raster-golden equivalence; the original golden
registrations are unchanged.
