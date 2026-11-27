# Reversed shape-border interpolation

The pinned `painting/shape_border_test.dart:61` case passed. It checks the
Dart runtime classes chosen by both of Flutter's interpolation entry points.
Incular has a single shared interpolation dispatcher and different internal
representation classes, so the registration is partial.

The focused native comparison runs all 25 pairs of rounded rectangle,
stadium, circle, oval, and star. At 0.25 forward and 0.75 in reverse the
same category of intermediate geometry is used. Their public containment
results agree on a grid in both a landscape rectangle and an offset
portrait rectangle. This compares outline geometry, not rendered pixels.
All five cases in the native shape-border lerp target passed.
