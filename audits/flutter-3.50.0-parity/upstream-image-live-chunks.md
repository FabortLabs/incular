# Live download chunks and image listener lifetime

Pinned `painting/image_stream_test.dart:179,207,235,266` passed.
The native base-completer fixtures receive both original progress
reports in order and do not replay a previously reported chunk when
a listener joins. The latest snapshot is still available through
`ImageStream::state()`. The progress stream does not wake the separate
frame-only observer.

The authored pinned and native controls cover duplicate reports,
pending stream handoff, and cancellation. Listener removal inside a
callback takes effect with the next report, not the already-captured
snapshot. Rust also releases callback captures outside the registry
borrow, so a capture is free to own and drop another subscription to
the same stream. Base and handoff observers form no strong cycle.

The two MultiFrame originals are partial: progress delivery matches
the pending native source, but its one-frame stream does not stand in
for Flutter's animated decoder. All 20 focused native tests and the
façade check without default features passed.

## Grouped listener identity

Exact pinned `painting/image_stream_test.dart:845` passed. The
native fixture reproduces all eight equality and hash comparisons:
image callback identity, optional progress, and optional error
callback identity all matter. The authored pinned probe also sends
progress followed by an error through a single listener. Its native
counterpart observes exactly that pair and verifies that dropping
the group unregisters both kinds of callbacks. Already-ready groups
receive the same decoded image synchronously as an `ImageInfo`.
The owned API still models one provider resolution; animation
playback is not claimed by this registration.
