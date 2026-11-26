# RenderAspectRatio sizing and intrinsic constraints

All six pinned `rendering/aspect_ratio_test.dart` registrations ran. The
four intrinsic cases (13, 34, 55, 76) and the live sizing matrix (125)
are paired. Finite opposite extents use the specified ratio for both
min and max queries. At infinity, they use the corresponding child's
extent rather than a speculative unbounded aspect-ratio layout. Tests
exercise the actual `IntrinsicWidth` and `IntrinsicHeight` callers with
a shrink child and with a fixed 90-by-70 child. The same keyed retained
aspect-ratio element is then resized through all the Flutter constraints:
tight host size, each finite cap, each singly unbounded axis, and
conflicting minimum sizes. An additional native control uses a tight
height and unbounded width, which must not be confused for tight axes.

The both-unbounded case (97) is partial. Flutter reports a framework
diagnostic and a follow-on VM error. Incular used to lay out the widget
at zero without a diagnostic; it now returns a single typed invalid
widget configuration error with the problematic ratio and both
unbounded axes. It does not manufacture Flutter's follow-on error.
The standalone layout helper can still choose a scale when the caller
has explicitly supplied a measured child.
