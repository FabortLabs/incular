# Haptic feedback platform messages

Both pinned `services/haptic_feedback_test.dart` originals
(`:11` and `:28`) passed. The native haptic service sends one
JSON method call to flutter/platform for each request. Plain
vibrate carries null; the seven named patterns carry exactly
Flutter's argument strings. The fixture decodes the payload
from the transport and asserts the channel, method, count and
arguments for each request.

Additional native controls check completion on an unhandled
optional channel and propagation of real host or platform
errors. Nine focused platform and codec tests passed. No
physical vibration was claimed: the embedder supplies the
transport, as the original tests do through their mock handler.
