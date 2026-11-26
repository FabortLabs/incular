# Semantics: accessibility channel notifications

Flutter 3.50.0's `semantics_service_test.dart:12` is paired over the
actual `flutter/accessibility` StandardMessageCodec envelope. The two
announcements carry view identity, different directions (LTR 1, RTL 0),
and omit default polite assertiveness but encode explicit assertive as
1. The API is an opt-in transport for Flutter-protocol embedders;
it does not make an unconnected platform speak.

The focused native adapter also encodes node-scoped tooltip, tap,
long-press, and focus and skips detached nodes. The exact
`semantics_node_send_event_test.dart:17` reference and native test
now pair the asynchronous error report. An attached node produces
no report while transport is pending, then exactly one carrying the
original exception and the semantics library and delivery context.
Native embedders install the framework reporter on the service
instead of mutating a process-global FlutterError handler. Rust
callers still receive the typed error. Detached nodes never send or
report. Three targeted adapter tests and six adjacent codec tests passed.
