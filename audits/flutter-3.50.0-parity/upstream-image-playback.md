# Deferred codec startup and frame playback

The seven exact pinned registrations below ran on Linux. The native
fixture uses an explicitly completed UI-thread codec and two actual pixel
resources (20×10 and 200×100) with independent frame durations.

| Flutter registration in `painting/image_stream_test.dart` | Native evidence | Status |
| --- | --- | --- |
| 99, 124 | No decode until both codec and listener exist; precisely one request whichever arrives first; codec stays alive pending completion. | Paired |
| 149 | Last listener drops the codec before the deferred decode completes, and that stale completion never publishes. | Paired |
| 354 | Single-frame decode publishes the shared image, releases the codec after dispatch, and never requests an app frame; late listeners replay synchronously. | Paired |
| 488 | The second image presents at 200 ms, not 199 ms; a zero-repeat animation releases the codec after the two frames without a third request. | Partial: explicit host clock |
| 533 | Removing the final listener stops scheduling and prefetching; a keep-alive preserves the last frame for replay on re-add; the codec releases after both owners are gone. | Partial: explicit host clock |
| 579 | Removing and re-adding while the first frame is pending drops that stale frame; only the newly decoded image is presented. | Partial: explicit host clock |

`ImagePlayback` exposes frame demand to the host rather than using a
background thread or continuous polling. It does not yet wire default
image providers, animation-format decoding, or desktop timer/dilation
integration. No wgpu paint-path changes.
