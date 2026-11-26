# Predictive back event messages

All eight pinned originals at `services/predictive_back_event_test.dart`
pass. The Rust ports round-trip actual StandardMethodCodec method bytes,
then validate the decoded fields. Edge 0 and edge 1 preserve (0,100) as a
swipe at zero progress, while zero and absent positions are back buttons.
Invalid progress (2) and edges (2) are rejected instead of clamped or
coerced. Rust reports typed errors where Dart raises assertion/range
errors. The convenience native constructor still sanitizes incidental
inputs; it is separate from parsing channel messages.

Two independently constructed events agree in equality, hash and
diagnostics. Moving the x coordinate by one changes each comparison.
Rust also checks equal hashes for signed zero. Hash numbers themselves
are not compared across languages. The authored Flutter probe pins the
exact diagnostic fields and verifies that standard start, update and
cancel bytes reach only the observer that accepts start. The Rust
integration additionally rejects an invalid update before dispatching
the following valid one. Thirteen focused Rust tests passed.

This is channel decoding and retained observer delivery, not an
Android-device callback or hardware back-button test. The host must
feed parsed events into its existing platform dispatch.

## Back observer failure isolation

Five exact `widgets/binding_test.dart` originals (:866, :1058, :1175,
:1225, :1270) pair on their asserted outcomes. Before the repair,
a panic in the first start observer escaped and skipped later handlers.
Start, progress, commit, cancel and ordinary back now isolate observer
failures and continue through the appropriate handlers. The predictive
ports decode the same standard method message before dispatch; each
asserts one typed failure and confirms the other observer was notified.

These checks compare failure count, callback stage and continued
delivery, not FlutterErrorDetails or native SystemNavigator failure.
Targets built with panic-abort cannot catch observer panics. Incular's
automatic nested navigator priority and cleanup-on-removal stay
unchanged. Eleven native tests passed, including the nested navigator
fallbacks.
