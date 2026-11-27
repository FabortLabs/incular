# Image ownership and ready-stream handoff

Exact pinned `painting/image_stream_test.dart:1039,1058,1092,1156,1169`
passed. In Incular the ready frame reports synchronous delivery only
if the listener attaches after the ready source is already installed.
The identical handoff after a pending listener preserves asynchronous
delivery semantics and invokes that listener once.

The native `ImageInfo` fixtures use an actual decoded 20 × 10 RGBA
image. Clone detection checks resource identity, scale and debug label:
the same pixels at scale 1 and scale 2 are not equivalent metadata,
and a newly decoded equal pixel array is not a clone. A cloned info
preserves logical size and shares the original 800 decoded bytes.
The final drop releases the shared resource; creation and disposal
events describe each info's own lifetime in debug builds. All 15
focused image stream and info tests passed.
