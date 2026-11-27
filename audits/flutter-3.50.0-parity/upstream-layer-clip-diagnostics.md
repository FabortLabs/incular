# Retained clip layer defaults and diagnostics

The four exact Flutter 3.50.0 `rendering/layers_test.dart` clip diagnostic
registrations passed. Rectangular clips default to hard edges, while
rounded rectangles, rounded superellipses and paths default to
antialiasing. All four report explicit antialiasing with isolation if
requested.

The native compositor now emits the actual retained layer's
`clipBehavior` in its normal tree dump using the typed diagnostic
property path. Its rounded and path convenience constructors default
to antialiasing instead of hard edges. The superellipse constructor
uses the existing native path contour, not a rectangular fallback.
Explicit coverage calls and the default rectangular clip remain
unchanged.

The four native paired cases and 55 focused layer, annotation and
clip geometry regressions passed. This checks retained behavior and
the requested display-list coverage, not physical GPU raster output.
Backdrop blur tile-mode diagnostics remain outside this batch.
