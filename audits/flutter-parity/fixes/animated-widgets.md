# Fixes: animated-widgets

Scope: switchers, implicit and transitions findings owned by the animated-widgets group. Records are in `animated-widgets.jsonl`. Nothing committed.

## Scenario results (after all changes)

- `switchers__`: 10/13 match. Still diverging: switchers-001, switchers-002 (blocked on the same tree-layout cause), switchers-007 (blocked on frame ordering).
- `implicit__`: 17/19 match. Still diverging: implicit-001 (slivers worker), implicit-002 (text engine). `implicit__curve_only_update_mid_flight` now matches.
- `transitions__`: 8/9 match. Still diverging: `transitions__text__default_style` (implicit-002).

## switchers-001: blocked
- Cause: outer box size comes from the child size observed on the previous layout, the same mechanism as switchers-002.
- Blocker: tree layout (`tree/layout.rs`, `tree/layout/behavior/containers.rs` SizeObserver) cannot resize a retained box after its child is laid out in the same pass.

## switchers-002: blocked (panic part fixed)
- Panic fixed: `AnimatedSize::animated` clamps to zero before `Size::new`, so overshooting curves no longer panic; `constrain` applies the box limits. Test: `an_overshooting_curve_clamps_the_size_instead_of_panicking`.
- Retarget frame lag still present (frame 11): same blocker as switchers-001.

## switchers-003: fixed
- Added `AnimatedSwitcher::animated_transition_builder(Fn(Widget, DoubleAnimation) -> Widget)`; each entry carries a `CurvedAnimation` with switch-in and switch-out curves. Scalar `transition_builder` unchanged. Test: `the_animated_transition_builder_receives_the_switch_animation`.

## switchers-004: invalid
- Flutter 3.41.7 `RenderAnimatedSize` only runs forward; `reverseDuration` has no observable effect. No change.

## switchers-007: blocked
- Cause: collection entry controllers tick in `update_compositor`, after layout in the runtime frame, so the removal lands one frame late. Trace: value 0 at 336 ms in both, presence differs.
- Blocker: needs a pre-layout tick for collection controllers (tree/runtime).

## implicit-007: fixed
- Cause: the displayed curve was read from the current widget on every build. Now the curve in effect is kept until the next target change, as `AnimatedWidgetBaseState` does. Test: `a_curve_only_update_keeps_the_running_curve_until_the_target_changes`. Scenario matches.

## implicit-002: blocked
- Cause: line-box height rounding for fractional Inter sizes in the text engine (`incular-text/src/engine.rs`). Not owned by this group.

**Text residual pass: fixed.** Same text-engine fix as text-011. `implicit__default_text_style__text_style` and `transitions__text__default_style` now match; `implicit__` is 19/19 and `transitions__` 9/9.

## implicit-003: deferred
- Directional `AlignmentGeometry` and `EdgeInsetsGeometry` inputs are missing from four implicit widgets. API surface too large for this pass.

## implicit-004: blocked
- Opacity always keeps child semantics (`tree/descriptors.rs`). Needs an Opacity policy field read by semantic projection.

## implicit-005: fixed
- `AnimatedPhysicalModel::shape(BoxShape)` added; applied immediately, as Flutter does. Test: `animated_physical_model_paints_its_shape` (paint output differs for circle and rectangle).

## implicit-006: blocked
- `DefaultTextStyle` carries only a `TextStyle`; paragraph options need the inherited scope and Text reads (`environment.rs`, text widget).

## transitions-001: fixed
- Added `SlideTransition::from_animation_in_direction(position, Option<TextDirection>, child)`; RTL mirrors dx. Test: `slide_transition_mirrors_the_horizontal_offset_in_rtl`.

## transitions-003: blocked
- `Transform` descriptor has no filter-quality field (`tree/descriptors.rs`, layout worker's `layout/basic/transforms.rs`).

## transitions-004: fixed
- Added `RelativePositionedTransition::from_optional_rects`; a `None` endpoint is the zero rectangle. Test: `relative_positioned_transition_scales_a_null_endpoint_from_zero`.

## transitions-005: deferred
- Needs a renderer-neutral `Decoration` interpolation type covering `BoxDecoration` and `ShapeDecoration` with Flutter's mixed-type rules.

## transitions-006: blocked
- Needs `SliverFadeTransition::from_animation(DoubleAnimation, child)` in `scrolling/sliver_fade.rs` (slivers worker). Not edited here.

## transitions-007: invalid
- Rule 8: the Rust-first equivalent is the closure-driven `AnimatedBuilder`/`ListenableBuilder`; no subclassable base.

## transitions-008: fixed
- `fixed_cross_axis_size_factor` asserts non-negative instead of clamping. Test: `size_transition_rejects_a_negative_fixed_cross_axis_factor` (`should_panic`).

## transitions-009: fixed
- Added named `RelativeRectTween` (`begin`, `end`, `lerp`, `animate`), exported. Test: `relative_rect_tween_interpolates_each_side`.

## transitions-010: invalid
- Rule 8: Flutter's `StatusTransitionWidget` has no framework subclass; `DualTransitionBuilder` and `AnimatedBuilder` cover status-driven rebuilds.

## Shared and related changes
- `OverflowBox` in `AnimatedSize` now uses `Alignment::TOP_LEFT`. Another worker's concurrent change made `OverflowBox` align its child (`tree/layout/behavior/core.rs`); `AnimatedSize` already applies its own alignment, so the alignment was applied twice. This fixed `the_child_is_aligned_while_the_size_differs` and the switchers `animated_size_top_left_alignment` divergence.
- `crates/incular-widgets/src/lib.rs`: additive exports only (`AnimatedSwitcherAnimatedTransitionBuilder`, `RelativeRectTween`).
- `crates/incular-widgets/src/animation_transitions.rs`: its uncommitted working-tree changes present at session start were lost. See the report for details; the file now equals HEAD plus this group's edits.

## Validation
- `cargo check -p incular-widgets --lib`: passes.
- `cargo test -p incular-widgets --no-fail-fast --test animated_size --test animated_switcher --test implicit_animations --test explicit_transitions --test animated_builders --test animated_cross_fade`: all pass (8 + 3 + 5 + 6 + 9 + 14 tests).
- `cargo clippy -p incular-widgets --lib --test <owned six> -- -D warnings`: passes. The `--all-targets` run stops on another worker's `tests/shape_decoration.rs` (redundant closure).
- `rustfmt --edition 2024` on every owned file: applied.

## Frame-order pass

- **switchers-002: fixed.** `AnimatedSize` now sizes its box in the same layout as its child, after the child. A doc-hidden child-sized layout runs `RenderAnimatedSize.performLayout`'s state machine with the child's size, then `constrain(_animatedSize)` and `alignChild`. The build no longer reads the animation.
  - Files: `custom_layout.rs` (`CustomLayoutDelegate::ChildSized`), `tree/layout/behavior/containers.rs` (layout arm), `tree/widget/structure.rs` (widget type).
  - Test: `animated_size.rs::a_retarget_restarts_from_the_last_laid_out_size_in_the_same_layout`.
  - Harness: `switchers__animated_size_retarget` matches.
- **switchers-001: fixed** by the same change, since AnimatedCrossFade sizes through AnimatedSize. `switchers__cross_fade_reversed_different_sizes` matches.
- **switchers-007: already-fixed** by the residual pass's pre-layout sliver tick. `switchers__animated_list_remove_shifts_items` matched before any frame-order edit.
- Harness: `switchers__` 13/13, `implicit__` 19/19, `transitions__` 9/9 match.
