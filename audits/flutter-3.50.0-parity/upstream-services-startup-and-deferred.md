# Services initialization and deferred asset requests

Pinned `services/binding_lifecycle_test.dart:36` passes. Completing
initialization sends exactly `System.initializationComplete` on the
optional JSON `flutter/platform` channel with null arguments. The
native bridge does not report completion before the host replies.

Pinned `services/deferred_component_test.dart:11,34` pass. The port
sends one standard-codec method per operation on
`flutter/deferredcomponent`, with `loadingUnitId: -1` and the exact
component name. The future waits for host acknowledgement. A missing
optional service is a no-op, while the standard platform error
retains its message, code and details.

This pairs the framework-to-host boundary. It does not download or
install Android split assets on the audit machine, execute deferred
code, or claim the absence of later asynchronous disk removal.
Three new native tests and 17 nearby codec and lifecycle checks passed.
