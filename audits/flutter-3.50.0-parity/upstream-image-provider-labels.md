# Image provider provenance

The four pinned `painting/image_provider_test.dart` label originals
pass. The native FileImage stream retains its selected path, ExactAssetImage
retains its resolved bundle key, and MemoryImage produces a short label
stable for the byte allocation across a cold resolve, a warm cache hit,
and a cloned provider. ResizeImage appends the requested dimensions
without modifying the original stream.

A native asynchronous test checks labels still reach ImageInfo listeners
when a deferred stream follows a transformed image. The transformed
stream retains its label without retaining the completed source.
The Rust stream implementation does not claim Flutter codec subclass
parity or the same process-specific identity integer.
