# Fixes: frame-order

Records are in `frame-order.jsonl` (new ids). The updated records for switchers-001, -002 and -007 (`animated-widgets.jsonl`), gestures-006 (`gestures.jsonl`) and anim_core-001 (`anim.jsonl`) carry a "Frame-order pass" note, and their `.md` files have a "Frame-order pass" section. Nothing committed.

## anim_core-003: fixed
- Cause: frame stamps reached tickers, frame callbacks and the retained clock raw; there was no scheduler-wide dilation.
- Change: `scheduler.rs` adds `set_time_dilation` and `time_dilation` with Flutter's epoch rules (`_adjustForEpoch`, `resetEpoch`). `begin_frame` and `end_frame` dilate the stamp. The tree's `animation_now` advances by dilated intervals. Re-exported from `incular-animation/src/lib.rs` (additive, out of list).
- Twin: the Incular diagnostic twin of `anim_core__scheduler_callbacks_time_dilation` now calls `set_time_dilation(2.0)` and matches.
- Test: `scheduler_time_dilation.rs::dilation_slows_ticker_time_from_the_next_frame`.
- Limitations: per thread; gesture timing stays raw; not in the `incular` facade prelude.

## anim_core-005: fixed
- Cause: `with_vsync` controllers tick from the scheduler, outside the TickerMode check of retained ticker nodes.
- Change: `interaction.rs` syncs `set_ticker_muted` for vsync controllers at their `AnimationTicker` nodes, before and after the build.
- Test: `ticker_mode.rs::a_vsync_controller_follows_the_enclosing_ticker_mode`.
- Limitations: a vsync controller with no ticker node is not reached; a node removed while muted leaves the controller muted.

## Validation
- `cargo test -p incular-animation`: every test file passes.
- `cargo test -p incular-widgets` with `animated_size`, `animated_cross_fade`, `animation_ticker`, `ticker_mode`, `advanced_slivers`, `reorderable`, `scroll_pages_reorder`, `animated_switcher`, `explicit_transitions`, `implicit_animations` and `custom_layouts`: all pass.
- Harness (Incular side against existing Flutter traces): `switchers__` 13/13, `anim_core__` 7/7, `gestures__reorderable` 1/1, `implicit__` 19/19, `transitions__` 9/9, `slivers__` 14/15. The one sliver divergence is the keep-alive rect (slivers-004 residual), which was already there.
- `cargo clippy -p incular-animation --all-targets -- -D warnings` passes. Widgets clippy stops on another worker's `drag_drop.rs` type-complexity lint; it reports no lint in files changed here.
