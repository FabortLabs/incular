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
