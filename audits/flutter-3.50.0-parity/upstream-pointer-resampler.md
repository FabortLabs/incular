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
