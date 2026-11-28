# Flutter 3.50 rendering proxy comparisons

Pinned `packages/flutter/test/rendering/proxy_box_test.dart` :16,
:30, :81, :122 and :583 passed by exact name. Native checks run the
retained layout, hit-test, repaint and compositor paths. The initial
native ports found three distinct issues.

- Empty FittedBox skipped drawing the retained child but still called
  its painter. It now skips pending child paint while either measured
  dimension is zero. The comparison checks the actual callback, misses
  at the origin and a singular retained affine transform. Flutter's
  full 4×4 paint-transform API has no exact analogue, so :16 is partial.
- The physical model toggled between two widget types when PhysicalModel's
  elevation went from zero to positive. A retained root rejected that
  update. The same retained proxy now stays active, but a completely
  transparent shadow is a compositor passthrough and adds neither an
  effect command nor visual extent. The test counts zero, one, zero
  shadow effects over retained updates; it does not claim matching the
  Flutter ambient/spot shadow kernel.
- Equivalent PhysicalShape geometry incurred repeated painting from a
  fresh layout builder. The path is already local geometry, so a tight
  positioned painter adopts the retained surface size directly. Equal
  geometry does not repaint; changing a circle to a stadium does.
- The 200×200 FittedBox child in the 100×100 viewport emits the expected
  clipping coverage for default, none, hard-edge, antialias and
  antialias-with-save-layer modes.

## Layer identity during a real repaint

The pinned :375, :425 and :566–574 originals passed. At the original
10×10 bounds, both opacity variants and both FittedBox clip/transform
representations keep the same live layer after an explicit repaint.
The native test asserts that painting actually ran and compares retained
layer IDs, not log lines or equivalent pixels. A separate retained control
starts with an empty FittedBox and confirms the pending custom paint
runs exactly once after the parent first gets nonzero geometry.

## Offstage and opacity child paint eligibility

Pinned :853, :870, :883 and :900 passed by exact name. The retained
public `paints_child` query and the real paint traversal share one
eligibility predicate. Offstage, static opacity, live box opacity and
live sliver opacity report false for their child at the tested states.
A zero or rounded-to-zero alpha no longer calls the child painter.
`always_include_semantics` can still retain the label and its bounds.
The original first-painter counter now passes at 0, .001, .003, .1 and 1.

### Half-byte threshold in retained scenes

The pinned authored `opacity_paint_threshold_test.dart` verifies
both neighboring representable Rust inputs around half an alpha byte.
The renderer now applies the same threshold when flattening existing
cached pictures, not just when generating a new picture. It keeps the
opacity layer marker and ID, but omits its source commands and visual
bounds when the effective alpha is zero. The adjacent visible value
restores the same retained geometry. Targeted compositor, widget
opacity and hit/semantics checks passed. This is not a GPU screenshot
comparison.

## Opacity layer bytes and composition gates

All six pinned original opacity compositing cases at
`rendering/proxy_box_test.dart:346,356,365,386,399,412` passed.
The native cases inspect the real opacity group, nonzero child draw
work, and retained generation for zero, partial and full opacity.
A zero-alpha debug marker can retain a layer identity without any
source draw, bounds scan or isolation. Repainting the child at half
opacity updates its source generation under the same opacity layer.

The authored probe checks actual Flutter `OpacityLayer.alpha`
values across the first visible byte and the 0.1, 0.5, 0.7, 0.9,
0.99 and 1.0 inputs. Widget opacity now passes these same bytes
(as normalized values) to the retained renderer, rather than leaving
fractional values to be rounded only after compositing. An animated
opacity at 0.49/255 still reports that its child is eligible, as in
Flutter's raw animation-value query, while submitting no raster work.
The generic continuous effect API remains unchanged. Fifty-three
focused native tests and the authored Flutter control passed; no
GPU device was required or exercised.

## Empty physical layers and shadow bounds

Pinned `proxy_box_test.dart:65,144` both keep an empty physical layer
non-composited at elevation 0 → 1 → 0; the shape case exercises each
Flutter target platform. The mounted Incular regression checks the
same transitions in the display list wgpu consumes. It also grows the
same retained render object to a nonzero size and back: a shadow group
appears only while it has a drawable source.

Previously shadow inflation turned a zero-area source rectangle into
positive render bounds and submitted an unnecessary GPU effect group.
The compositor no longer inflates empty source bounds or emits a
PushDropShadow/PopEffect pair for them, though it keeps retained IDs
and still visits ordinary children. The comparison targets effective
compositing, not native layer class names or the look of the shadow.
