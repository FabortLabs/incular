# Deferred codec startup and frame playback

The twelve exact pinned registrations below ran on Linux. The native
fixture uses an explicitly completed UI-thread codec and two actual pixel
resources (20×10 and 200×100) with independent frame durations.

| Flutter registration in `painting/image_stream_test.dart` | Native evidence | Status |
| --- | --- | --- |
| 99, 124 | No decode until both codec and listener exist; precisely one request whichever arrives first; codec stays alive pending completion. | Paired |
| 149 | Last listener drops the codec before the deferred decode completes, and that stale completion never publishes. | Paired |
| 354 | Single-frame decode publishes the shared image, releases the codec after dispatch, and never requests an app frame; late listeners replay synchronously. | Paired |
| 387 | A completed decode waits for an app frame; the 200 ms predecessor is still visible at 100 and 199 ms; the 400 ms frame then follows. | Paired |
| 440 | The infinitely repeating codec wraps 20×10 → 200×100 → 20×10 at the original time boundaries, then releases only after the final listener drops. | Paired |
| 488 | The second image presents at 200 ms, not 199 ms; a zero-repeat animation releases the codec after the two frames without a third request. | Paired |
| 533 | Removing the final listener stops scheduling and prefetching; a keep-alive preserves the last frame for replay on re-add; the codec releases after both owners are gone. | Paired |
| 579 | Removing and re-adding while the first frame is pending drops that stale frame; only the newly decoded image is presented. | Paired |
| 638 | Two listeners share the first decoded frame; removing one leaves the survivor's scheduled frame and codec intact. | Paired |
| 690 | Removing the last listener clears the armed deadline and releases the decoder immediately. | Paired |
| 728 | A 200 ms frame with 2× dilation does not advance after 200 ms of wall time, but does advance and prefetch after 400 ms. | Paired |

`ImagePlayback` exposes frame demand to the host rather than using a
background thread or continuous polling. The scheduled constructor runs
through the production Incular frame scheduler. The runtime exposes the
earliest scheduler deadline to its host; desktop's existing event loop
uses that deadline rather than polling. Both scheduler and runtime
focused tests passed. These are mock-codec parity cases like Flutter's
originals, not proof of stock animation-format decoding or default
provider and widget integration. No wgpu paint-path changes.

## Real WebP frames

The two checked-in authored bitstreams exercise a finite two-play
animation with 200/400 ms delays and an infinite partial-frame
animation. The pinned Flutter probe checks exact straight RGBA bytes
through two loops and tests an explicit rectangle disposal bit.
Six native decoder tests passed, including the same pixel assertions,
static-image detection, invalid input, and pre-allocation dimension
rejection. The original third-party animation compositing path rounded
an opaque blue channel from 255 to 254. Incular reuses the WebP decoder
for each subimage and composites it locally, preserving opaque
channel values and clearing the canvas at a loop boundary. The
synthetic subimage header borrows encoded bytes, and the decoder need
not cache any prior full output frames. This is an opt-in decoder
that can run on a blocking pool; default provider integration remains
open. Authored fixtures add no original registrations.

## Mounted image timing and reduced motion

Five exact `widgets/image_test.dart` registrations executed. The
retained runtime tests use the real scheduled playback and decode a
real WebP for each frame. They verify numbered frame-builder calls,
the actual image paint commands, and no extra rebuild on an idle
frame.

| Registration | Retained runtime evidence |
| --- | --- |
| 1076 | Builder sees absent → 0 → 1, while render output goes empty → red → blue. |
| 1220 | TickerMode off freezes the widget while another listener receives two further frames. Re-enabling replays the latest frame synchronously into the same widget state. |
| 1294 | Scoped reduced motion pauses a mounted image; lifting the setting replays the current frame, then allows the next decode to display. |
| 1942, 2012 | If either setting starts disabled, the first real frame paints, and subsequent host frames do not rebuild or prefetch another frame. |

The original fixtures use a custom Flutter stream completer.
`Image::playback` is its native counterpart for an already-resolved
animation. Each mount owns its subscription and reads the nearest
MediaQuery animation setting without making unrelated media aspects
dependencies. Ordinary one-shot providers still pass their focused
suite. Automatically recognizing animated input in the standard
providers and supporting other animation containers remain open.

### Worker-backed encoded playback

`RuntimeSpawner::load_animated_webp` opens a source on the existing
blocking worker pool. Per source it decodes only one frame at a time
and returns each owned result to the UI thread before scheduling
presentation. The native runtime fixtures verify red/blue draws at
199/200 ms, invalid encoded input reaching the mounted error builder,
and a dropped source not being retained by an unfinished initialization.
The stream preserves a successfully decoded frame separately from
a later decode failure. These are authored checks and add no
registration credit.

### Standard memory provider

An in-memory WebP declaring animation now uses the window's worker-backed
playback in `Image::memory` and `Image::provider(MemoryImage)`. Still
images continue through the existing borrowed cache path. Equal encoded
bytes and effective cache settings preserve an existing mounted session
through rebuilds, and the worker cannot retain an unmounted widget.

The authored `Image.memory` probe initially expected `cacheWidth: 2`
to resize a 4×4 multi-frame source. The pinned Flutter runner instead
produced 4×4 pixels: `ImageDescriptor::instantiateCodec` passes target
dimensions only to its single-frame codec. The native path preserves
the encoded multi-frame canvas and the requested width still contributes
to resolution identity. Both lanes now verify red until 199 ms and
blue at 200 ms. Ten native animated-image cases and eight regular
provider cases passed. This probe adds no original coverage credit.

### Callback and error lifecycle

The pinned reference ran seven exact registrations at lines
91, 297, 325, 771, 806, 909, and 985 of `painting/image_stream_test.dart`.
The paired native cases intercept decoder errors with a supplied
error callback, keep one duplicate group alive when another is
removed, and replay a one-frame decode from a passive keep-alive
without asking the codec again. Authored controls exercise reentrant
last-listener callbacks, explicit removal, and last-good-image
retention during a later decode failure. All 30 scoped native
listener/playback tests passed.

The keep-alive original remains partial: both lanes fire the last
listener callback and keep the decoder without publishing more
frames. Flutter nevertheless schedules a no-op transient callback
for a decode already underway; Incular drops its stale completion
without waking the host. Native deferred initialization, transport,
and frame errors reach listeners or can be replayed, but the three
originals testing Flutter's app-global uncaught-error reporter
remain partial.

Matching live animated memory keys now share one decoder and the same
image handles across mounts in a window. The active index stores only
weak resolution records, not strong encoded bytes or prior frames;
closing the last mount releases the source. A different cache width
keeps a separate resolution even where the animated engine ignores
that width for decoding. The original Flutter probe shares the same
`Uint8List` object; a separate equal byte array remains a distinct
Flutter key. Incular intentionally keeps its existing content-based
memory-provider identity for separate equal byte allocations. The
authored Flutter and native checks assert handle identity on real
painted frames; all eleven native animated widget tests passed.
