# LayoutBuilder stateful sizing and unmatched debug hooks

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All five selected originals passed. A stateful child can resize
its retained LayoutBuilder from 10×20 to 100×200 (:109).
A stateful parent changing its tight size gives the same successive
limits and child geometries (:216). Both are fully paired.

Changing only DefaultTextStyle (:378) rescales the text and its
LayoutBuilder without re-running the non-reading inner Builder.
Rust uses the real default font with an explicit line-height
factor to compare 10 to 100 height and a larger width. Flutter's
Ahem-based exact square glyph widths are not asserted in Rust;
partial. All eight focused Rust tests pass.

Dart render-node debugNeedsLayout (:352) and the subclassable
updateShouldRebuild paint policy (:813) are reference-only.
The Rust public builder revision offers a different API; this
does not simulate either check with an unrelated flag.
See `flutter350_layoutbuilder_dependencies`.
