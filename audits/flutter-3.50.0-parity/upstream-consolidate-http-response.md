# Streamed HTTP response bytes

All ten pinned `foundation/consolidate_response_test.dart` originals
passed in the native Flutter tester. The Incular counterpart joins
the actual asynchronously supplied bytes even when Content-Length
is missing or too small. It incrementally inflates still-compressed
GZIP chunks by default and returns identical compressed bytes when
auto-decompression is off.

Progress reports every input chunk in order, uses the compressed wire
byte counts when appropriate, and suppresses the stale declared
length when a client already decompressed the response. The stream and
callback failures keep their original errors and cancel consumption
before another chunk is read. Native controls observe the dropped
stream and require a second poll for a pending transport. Seventeen
targeted native response, asset network and license checks passed.
