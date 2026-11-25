# Semantics: accessibility channel notifications

Flutter 3.50.0's `semantics_service_test.dart:12` is paired over the
actual `flutter/accessibility` StandardMessageCodec envelope. The two
announcements carry view identity, different directions (LTR 1, RTL 0),
and omit default polite assertiveness but encode explicit assertive as
1. The API is an opt-in transport for Flutter-protocol embedders;
it does not make an unconnected platform speak.

The focused native adapter also encodes node-scoped tooltip, tap,
long-press, and focus and skips detached nodes. The source transport
error and its send context survive an asynchronous failure. This is
a partial pair of `semantics_node_send_event_test.dart:17`: the Rust
caller receives a typed error, not a global FlutterError publication.
Both exact Flutter originals, three Rust adapter tests, and six
neighboring codec tests passed.
