# Debug action timing

Pinned `foundation/debug_test.dart:27` and `:39` pair. Enabled
debug timing writes one line after a successful action's own output
and reports the result unchanged. A failure that wakes asynchronously
still emits exactly one timing line and keeps its original typed
error. A disabled or normally abandoned operation does not log a
completion, and an unwinding Rust panic keeps its original payload.

Native callers provide the log sink and the future begins timing on
its first poll. Flutter starts the async function at invocation and
uses its global debugPrint sink. Release builds leave timing
disabled. Both pinned originals and ten focused native tests passed.
