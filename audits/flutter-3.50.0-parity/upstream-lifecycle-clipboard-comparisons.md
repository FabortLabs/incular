# Upstream suspension and typed clipboard comparisons

Flutter 3.50.0 `ab2912035e`, Incular `c647cb85`. Five selected official
registrations passed Flutter. The committed, focused Rust target
`flutter350_platform_lifecycle_clipboard_parity` recorded three
matches and two expected failures.

- `scheduler/ticker_test.dart:154`: Ticker running before suspension
  should stop ticking. Incular acknowledged suspended lifecycle state,
  but its active ticker still reports ticking.
- `scheduler/ticker_test.dart:178`: A ticker started while suspended
  remains quiet until resume on Flutter. Incular reaches callback
  counts 1 and 2 before resume; the fixture pumps only frames the runtime
  requested, not arbitrary frames while paused.
- `services/clipboard_test.dart:17,26,46`: typed plain-text read,
  absent read, and a single plain-text write match the host boundary.
  No real clipboard, device suspension, malformed dynamic payload,
  method-channel transport, or reversible detach equivalence is claimed.

Run `cargo test --locked -p incular-runtime --test flutter350_platform_lifecycle_clipboard_parity`.
Flutter originals ran as two separate exact-name filtered test files.
Raw logs are in `.tmp/flutter350-full-audit/platform-lifecycle-clipboard-upstream-*.log`.
