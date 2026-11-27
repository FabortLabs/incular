# Buffered inbound platform messages

Reference: pinned Flutter 3.50.0, engine `lib/ui/channel_buffers.dart`
and framework `lib/src/services/binding.dart`.

- The exact `services/channel_buffers_test.dart:24` registration passed.
  The native comparison pushes the same bytes (`bar` on `foo`) before
  registering a handler. The Rust handler and host reply are both still
  untouched when registration returns; a microtask turn delivers the
  original bytes and sends a null reply.
- The exact `services/default_binary_messenger_test.dart:18` registration
  passed. Native counts start at zero and stay at zero immediately after
  registration, then become one inbound and one outbound. The test
  separates the handler turn from its delayed reply and pumps again to
  verify the counts do not change. The responder is a one-shot closure.
- Two authored pinned controls also passed: overflow replies null for the
  oldest message, incoming messages cannot overtake a drain, a handler's
  own microtask runs before the next message, removal halts the drain,
  replacement resumes it, and live input becomes immediate after the
  drain. Shrinking discards the oldest entry and preserves the
  distinction between null and empty bytes across channels. Four focused
  native tests cover these cases.

The native receiver takes a host-local microtask scheduler and explicit
reply closures; it is not a process-global binding, Dart Zone, or new
wire transport. No parity credit is claimed for Flutter's test-only mock
handler introspection or for built-in host or plugin wiring, the control
channel protocol, or Dart's exception-reporting wrapper.
