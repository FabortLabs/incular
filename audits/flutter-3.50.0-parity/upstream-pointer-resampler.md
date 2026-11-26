# Pointer event resampling

### Interpolation and lifecycle transitions

Five pinned `gestures/resampler_test.dart` registrations passed (65,
355, 392, 471, 712). The native test checks emitted event count after
each sample, exact positions and deltas, sample timestamps, and
pressed-button metadata on synthesized moves. It covers hover moving
through press and release, same-timestamp quick taps, repeated
samples, fractional interpolation, large clock steps, and a final
move delivered before release/removal. Positions are interpolated;
hardware metadata is carried from the underlying event.

The resampler takes the host's monotonic clock and typed raw pointer
events, with Enter/Exit representing tracking start/end. It is
opt-in per stream and does not add latency to the default runtime
input path. Flutter's metrics that are not present in Incular's
raw pointer type were not asserted by these registrations.

### Streaming, skipped samples and lookahead

The other five registrations (179, 528, 599, 648, 773) passed.
Incrementally queued input yields the same point/delta trace as
preloaded input. A frame may coalesce motion across more than one
contact but keeps every press, release and tracking transition.
Stopping forwards each still-queued event unchanged and clears the
track. A supplied next frame can process a release or removal early
while keeping its interpolated position on the current sample, not
jumping to the future hardware event.

### Opt-in runtime touch delivery

Both pinned `gestures/gesture_binding_resample_event_on_widget_test.dart`
registrations (13, 109) passed. The native fixture mounts a real
Listener, queues the original eight samples converted from physical to
logical pixels, then uses a 5 ms lookbehind at 10, 20 and 30 ms frames.
The listener receives one Down, two interpolated Moves, and one early
Up, with the same timestamps, positions and deltas.

Touch resampling is disabled by default. Enabling with no custom delay
uses Flutter's 38 ms lookbehind. Mouse and stylus continue to dispatch
immediately. Disabling clears the runtime polling deadline and delivers
pending events with their original times; the test checks that a later
frame cannot replay them. The runtime uses its existing host frame
deadline interface rather than spawning a separate timer per contact.

### Sampling deadlines without a redraw

The pinned `gestures/gesture_binding_resample_event_test.dart` scenario
passed. Its local `testResampleEvent` wrapper registers the single named
scenario on line 64; the catalog now points there rather than to the
generic forwarding call on line 55. The denominator is unchanged.

The native runtime dispatches Down and Move in its first two frames.
A host-facing deadline poll then dispatches the remaining events at
the same 16,667-microsecond ticks without visiting layout or presenting
a frame. Late polls use the latest aligned tick; after release there
is no remaining deadline. A native control verifies that increasing
the lookbehind cannot replay a finger's movement backwards. The
application bridge exposes the same behavior per window to embedders.
