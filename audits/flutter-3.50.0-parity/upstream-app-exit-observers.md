# Application-exit observer dispatch

The two pinned `widgets/app_lifecycle_listener_test.dart` exit-request
registrations are paired. An inbound engine query reaches a registered
listener and replies with its decision on the existing JSON method
channel. If one asynchronous listener is pending, another disposed
before its turn is never called.

An authored Flutter probe also verifies that a cancellation cannot skip
later observers, an observer error is reported without aborting dispatch,
and a listener added during a pending notification is first visible on
the next request. The native route matches all three properties,
reports both callback and future panics, and leaves unrelated methods
available to the embedder.

This is a Flutter-compatible application binding, not a change to native
process shutdown. Four focused observer tests and the existing exit
and lifecycle targets passed.
