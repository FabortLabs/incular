# Foundation bit fields

Both pinned `foundation/bit_field_test.dart` registrations (lines 12 and 43)
passed on the Dart VM. Incular's `BitField<T>` now reproduces the unset
initial state, independent updates and clears, both reset values, and both
filled constructor values through actual enum indexing. Its named setter
is the Rust equivalent of Flutter's bracket assignment. The native suite
also checks index 61, rejection of index equal to the length, zero length,
and rejection of lengths above 62. The constant is the VM's 62-bit mask.

Flutter skips these registrations in JavaScript, where its stub throws.
Incular uses a `u64`, not a JavaScript number, and deliberately does not
introduce that stub. Rust bounds checks reject invalid access even in a
release build. No browser runtime ran in this batch.
