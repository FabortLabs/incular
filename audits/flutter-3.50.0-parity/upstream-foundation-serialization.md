# Foundation binary serialization buffers

All fifteen exact native-host registrations in
`foundation/serialization_test.dart` passed on pinned Flutter 3.50.0.
Seventeen focused Rust tests passed through the new
`incular::core::serialization` API.

The public writer starts at eight bytes unless a positive capacity is
specified; finalization is one-shot and moves out the written bytes
without copying. Scalars support host and explicit byte order. Integer
scalars are packed, float64 scalars align to eight, and typed arrays
align to their own element width. The tests inspect actual bytes and
zero padding, as well as read results for negative integers and NaN.

The authored `foundation_buffer_alignment_test.dart` compares an exact
mixed-endian 16-byte record. Rust rejects truncated and overflowing
read requests before advancing the cursor. Typed numeric arrays decode
to owned Rust values, rather than Dart's mutable views into shared bytes;
the byte-list read itself borrows the input. These are native host
comparisons, not Flutter Web int64 behavior.
