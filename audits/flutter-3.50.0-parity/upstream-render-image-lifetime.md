# RenderImage constraints and image lifetime

Pinned Flutter 3.50.0 `packages/flutter/test/rendering/image_test.dart`.

- **:154 Null image sizing.** All four sizes match. An unset dimension
  takes the parent's minimum; a specified dimension is clamped before
  being made tight. There is no raster or destination blend without an
  image.
- **:18 Image sizing, partial.** All nine square, wide, and tall sizing
  checks match, including increasing a second axis to keep the aspect
  ratio when constraints allow it and the exact result when they do
  not. The Dart `RenderImage.toStringDeep` snapshot is not shared.
- **:229 / :246 Image disposal and reapplication, partial.** Equal
  decoded image handles keep the retained render object, layout,
  picture, and image. Clearing it keeps the render identity, removes
  the raster from the next frame, and releases the image when its
  caller's handle is dropped. A cloned Rust handle shares pixel
  ownership through an Arc; Dart's exact handle stack counts are not
  compared. Previously a root RawImage changing to an empty handle
  would become an incompatible SizedBox and panic.
- **:264 Image disposal, partial.** Dropping the Rust tree frees
  the retained decoded pixels if no published display list or caller
  still owns them. Rust cannot inspect the disposed render object's
  null getter, and does not emit Dart's debug handle stacks.

The native `image_layout` and `paint_options` targets passed 44
tests. The lifetime probes use real decoded resource ownership, not
only whether image commands disappear.

## Source replacement invalidation and image hit edges

Replacing a source with another of the same decoded dimensions repaints
without changing layout. Changing dimensions still remeasures an image
with intrinsic sizing; two explicit widget dimensions keep their size
when replacing, clearing, or restoring the source. In the pinned
authored case these are the actual RenderImage layout and paint flags.
The native case checks real paint handles, render identity, and phase
counts before and after each frame.

An image owns the half-open layout box, even if no pixels are loaded.
The right and bottom edges and zero-size boxes are not image hits.
The authored Flutter and native hit-test probes exercise those edges;
no extra upstream registration is counted for either authored case.
