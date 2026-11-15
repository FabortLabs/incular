# Fixes: animated-widgets-2

Group `animated-widgets-2`. Records are in `animated-widgets-2.jsonl`. Nothing committed.

## transitions-006: fixed
- Cause: `SliverFadeTransition` accepted only an `OpacityController`.
- Change: `SliverFadeTransition::from_animation(impl Into<DoubleAnimation>, sliver)`. The controller constructor is kept. The animation path ticks its controllers each frame and lays children out at the clamped value.
- Test: `implicit_animations.rs::sliver_fade_transition_follows_an_animation_value`.
- Harness: `transitions__sliver_fade__opacity` matches.

## transitions-003: blocked
- Cause: `Transform` has no sampling control. It paints as `PushTransform` vector commands, and the compositor `LayerKind::Transform` holds only a matrix.
- No descriptor field was added, because a stored but unapplied option would be a dead API.
- Blocker: `incular-rendering` and `incular-wgpu` (sampling on transform paint and layer commands), plus tree threading. Outside this group's files.
- Harness: `transitions__paint__decoration_matrix` matches, but the recorder cannot observe sampling.

## transitions-005: fixed
- Cause: `DecoratedBoxTransition` took `TweenAnimation<BoxDecoration>` only.
- Change: `Decoration` enum (`Box` | `Shape`) with `Lerp`. Same kinds interpolate; mixed kinds switch at t<0.5. The constructor accepts `impl Into<TweenAnimation<Decoration>>`, with From impls for both tween kinds, so existing callers compile.
- Limitations: shape gradient, image and shape switch at the midpoint (`ShapeBorder` has no lerp). `DecorationPosition::Foreground` applies to box decorations only.
- Tests: `explicit_transitions.rs::decorated_box_transition_animates_a_shape_decoration`, `decorated_box_transition_switches_between_decoration_kinds_at_the_midpoint`.
- Harness: `transitions__paint__decoration_matrix` matches.

## implicit-003: fixed
- Cause: implicit layout widgets stored physical `Alignment`/`EdgeInsets` only.
- Change: `AlignmentGeometry` and `EdgeInsetsGeometry` (Physical | Directional). `TweenPass` carries the ambient direction, read from the build context in `ImplicitState::update`. Targets resolve with it; build fallbacks resolve with `context.text_direction()`. Setters accept `impl Into<...>`.
- Widgets: AnimatedAlign, AnimatedPadding, AnimatedContainer (alignment, padding, margin, transform_alignment), AnimatedFractionallySizedBox (alignment).
- Limitation: a direction change after a target was set is re-resolved only at the next target change.
- Tests: `implicit_animations.rs::animated_align_resolves_a_directional_alignment_with_the_text_direction`, `animated_padding_resolves_directional_insets_with_the_text_direction`.
- Harness: `implicit__` 19/19 match.

## anim_curves-004: fixed
- Cause: tween endpoints were non-optional.
- Change: `NullableTween`, `OptionalTween<T>`, aliases `OptionalColorTween`/`OptionalSizeTween`/`OptionalRectTween`, and `Tween for Rect`. A missing end scales the present one as Flutter's `Color.lerp`, `Size.lerp` and `Rect.lerp` do.
- Tests: `incular-animation/tests/flutter_curves.rs` (three tests, one per type).
- Harness: none (source finding).

## Validation
- `cargo check -p incular-animation -p incular-widgets --lib`: passes.
- `cargo check -p incular-parity-harness --tests`: passes (one warning in another worker's `harness/incular/tests/frames/text.rs`).
- `cargo check -p incular --tests`: passes.
- `cargo test -p incular-animation --test flutter_curves`: 7 passed.
- `cargo test -p incular-widgets --test explicit_transitions --test implicit_animations --test animated_builders`: all pass, including the new tests.
- `cargo clippy -p incular-animation --lib --test flutter_curves -- -D warnings` and `cargo clippy -p incular-widgets --lib --test explicit_transitions --test implicit_animations -- -D warnings`: pass.
- Scenarios: `harness/run-frames.sh transitions__` 9/9 match; `implicit__` 19/19 match.
- rustfmt `--edition 2024` applied to my owned widget files and test files.

## Shared-file edits
- `crates/incular-animation/src/lib.rs`: additive export of the optional tween names.
- `crates/incular-widgets/src/lib.rs`: additive exports `Decoration`, `AlignmentGeometry`, `EdgeInsetsGeometry`.
- Stray empty file `.tmp/tween_add.rs` and log `.tmp/frames-transitions.log` were created by me and deleted.
