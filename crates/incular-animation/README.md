# incular-animation

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Curves, interpolation, tweens and animation physics. |
| API class | application; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Available; interruption contracts consolidate in G. |

Renderer- and platform-independent animation primitives, including durations,
curves, tweens, controllers, and animation progress values.

`AnimationController` is cloneable and is ticked only with a runtime-supplied
monotonic `Instant`; it creates no per-widget timer or event loop. It currently
supports forward/reverse/stop/reset, linear/ease-in-out curves, and interpolation
for `f32` and `Offset`.

### Frame clocks

`current_frame_time()` returns the active frame's time adjusted for
animation dilation. Use `current_system_frame_time()` when you need
the most recent raw host timestamp, even between frames.

`schedule_forced_frame()` requests a frame even while normal scheduling
is paused. `Ticker::set_force_frames(true)` does the same for each new
tick request. Use these sparingly: an otherwise idle host may keep
producing frames, and a fully suspended OS may not respond.

`Priority` values describe the idle, animation, and touch bands. The
priority offset in each addition or subtraction is limited to
`Priority::MAX_OFFSET`; successive offsets can accumulate. This is an
ordering value, not a task executor.

Use `debug_assert_no_time_dilation(reason)` to catch time scales that
were left altered during development. `try_assert_no_time_dilation(reason)`
also checks release builds and returns a typed error instead of panicking.

Use `incular::animation::scheduler::schedule_task` for short UI-thread
callbacks, not blocking or background work. The runtime runs one eligible
task before each frame. By default, tasks below `Priority::ANIMATION`
wait until there are no pending transient callbacks. You can await the
result, or drop the future to let the callback run without observing it.

In debug builds, `scheduler::set_scheduler_debug_flags` can mark the start
and end of each frame. `try_assert_all_scheduler_vars_unset` reports whether
either banner remains enabled.
