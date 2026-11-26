# Foundation debug print formatting and throttling

All three pinned `foundation/print_test.dart` originals pair.
Synchronous wrapping emits one string with internal newlines.
Throttled wrapping emits individual lines. Both preserve words,
skip stack trace lines, and print an absent value as `null`.
The native fixture checks every original width from 0 through 13
and the continuation indentation of numbered list items.

The buffer charges 12 Ki UTF-16 units per burst but does not
truncate an oversized log line. Later lines wait for one
one-second deadline; additional messages cannot move that
deadline forward. The native host supplies the clock and sink
and arms the returned deadline. It need not poll a frame, start
a worker or sleep to flush the buffer. Both probes also use
emoji to distinguish UTF-16 units from bytes and scalar counts.

Eleven focused Rust checks, including the foundation license
regressions, and two authored Dart checks passed. Rust returns
the whole blank line defensively even for wrap width zero. This
does not automatically replace the native tracing sink or expose
the Dart `debugPrintDone` future; hosts can query whether the
buffer is drained.

```sh
cargo test --locked -j 1 -p incular-core --test flutter350_debug_print --test flutter350_licenses -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe debug_print_buffer_test.dart
```
