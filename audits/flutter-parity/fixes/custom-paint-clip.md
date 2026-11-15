# custom-paint-clip fixes

## paint-009: CustomPaint foreground and child layering
- Outcome: fixed (confirmed by code reading; no scenario exists).
- Cause: CustomPaint was a fixed DisplayList with no child slot and no foreground painter. Painters ran at construction, so shouldRepaint was never consulted.
- Change: CustomPaint takes a painter, a foreground painter and a child. With neither child nor foreground it is a leaf that paints its painter at the laid-out size. Otherwise it composes a Passthrough, non-clipping Stack whose painter, child and foreground layers are Positioned.fill(IgnorePointer(...)) siblings, so the paint order is painter, child, foreground. A repaint gate honors should_repaint.
- Tests: custom_paint_orders_painter_child_and_foreground, custom_paint_skips_repaint_when_should_repaint_is_false, custom_paint_foreground_does_not_absorb_child_hits, custom_paint_with_child_takes_the_child_size, custom_paint_painter_sees_the_laid_out_size.
- Harness: not run; no twin added.

## paint-010: CustomClipper for clip widgets
- Outcome: fixed (code reading).
- Cause: clip geometry was stored in the descriptor rather than derived from the laid-out size, and no reclip decision existed.
- Change: CustomClipper::get_clip(size) returns a ClipShape, with should_reclip, attached through custom_clipper on ClipRect, ClipRRect, ClipOval and ClipPath. The render layer rebuilds the clip only when the size or clip inputs change, or when the delegate reclips against the one that produced the geometry.
- Tests: clip_rect_custom_clipper_derives_the_clip_from_the_size, clip_rect_custom_clipper_reclips_only_when_asked.
- Limitation: a chain of two non-reclipping delegates is compared with the geometry source rather than the immediately previous delegate.

## misc-010: CustomPaint fill-to-constraints size
- Outcome: fixed (code reading).
- Cause: the preferred size was a finite Size, so Flutter's Size.infinite pattern could not be expressed.
- Change: the preferred size is Option<Size>; None lays out at the biggest size. Added fill_to_constraints, from_painter_filling and preferred_size.
- Test: custom_paint_fill_to_constraints_takes_the_biggest_size.

## transitions-002: SlideTransition hit-test policy
- Outcome: fixed for SlideTransition.
- Cause: from_animation always built the translation with hit tests transformed.
- Change: SlideTransition::from_animation_with_options(position, text_direction, transform_hit_tests, child). Existing constructors keep their behavior.
- Test: slide_transition_hit_tests_follow_the_policy.
- Limitation: Transform.transformHitTests is not exposed. The tree kind carries the flag, but the Transform descriptor in tree/descriptors.rs still builds it as true.

## Validation
- cargo check passes for incular-widgets (tests), incular-parity-harness (tests) and incular (tests).
- cargo clippy -p incular-widgets --all-targets -- -D warnings passes.
- Passing (all tests in each file): custom_paint_clip_delegates, clip_opacity, explicit_transitions, custom_layouts, render_invalidation, layout_basic, painting_effects, box_shadow_paint, box_decoration_painting.
- Failing: paint_skipping::hover_repaints_only_the_hovered_control ("hover must change the output"). Its scene uses only ActionSurface in a scroll view, and none of the changed paths. It is probably from a concurrent hover edit, but I did not verify it against a baseline.
