# Foundation stack-frame parsing

Eleven pinned `foundation/stack_frame_test.dart` registrations were compared
against the public `incular_core::StackFrame` parser. The focused native
target checks the exact 14-frame VM, 41-entry async VM, 10-frame missing-column,
and 14-frame source-mapped Web debug inputs. It checks every field, including
frame source, anonymous closures, named/default constructors, and missing
coordinates. A Web-format string is not a browser or Wasm run.

The package stack's exact async-gap marker is canonicalized to the VM
suspension frame. Near misses and other unrecognized lines do not become
frames. The elision marker, an HTTP constructor without a class, and the
complete mangled package stack are tested separately. A supplemental
Flutter probe checks three path spellings the original tests' equality does
not exercise: the HTTP leading slash, the directory in one `file:` frame,
and the underscore in `isolate_patch.dart`. Incular preserves those actual
parsed values; equality and hashing omit scheme, path, and the constructor
flag, as Flutter does. Compiled Web stack parsing has native-only evidence.

The text behind the Dart `StackTrace.fromString` wrapper passes, but Incular
does not emulate the Dart object wrapper. Native stack overflow recovery,
live runtime error capture, and diagnostic console filtering are separate
scenarios not credited here. Incular skips malformed numbered external
frames rather than asserting inside error reporting.
