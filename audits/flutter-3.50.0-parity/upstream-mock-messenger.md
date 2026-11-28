# Identity and dispatch of platform message mocks

Reference: Flutter 3.50.0 at `ab2912035e160d9cad6306df307576da6128ebdb`.

The native test transport checks the exact registered handler rather than
its channel name or type alone. A missing mock matches `None`; a duplicate
handle matches the installed callback, but a distinct closure with the
same behavior does not. Removing the mock restores the fallback. The
transport preserves null, empty, and nonempty binary messages, waits for
an asynchronous reply, and can adapt an actual JSON method call without
losing the registered handle's identity.

Three pinned originals passed: `default_binary_messenger_test.dart:45`,
`platform_messages_test.dart:14`, and `platform_channel_test.dart:301`.
All five focused native mock tests and sixteen adjacent inbound/method
channel tests passed. This is a testing overlay; it does not replace any
deployed platform transport.
