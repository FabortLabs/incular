# VM-sized integers in message codecs

All five pinned `services/message_codecs_vm_test.dart` originals
passed. The native JSON codec round-trips the signed VM extremes
without converting through floating point, including the exact
large negative list value. The standard codec uses the 32-bit
wire tag only inside the closed signed 32-bit interval, and emits
the exact little-endian payload on the audited host for the signed
64-bit boundaries. An explicit wrapping operation reproduces the
Dart VM arithmetic on the two overflow-boundary fixtures. The
mixed list retains a distinct Int64List with both signed endpoints.

The five new native fixtures exercise the actual encode/decode
paths; all 23 tests in the selected platform run passed. Browser
behavior at these VM-only integer literals is not claimed.
