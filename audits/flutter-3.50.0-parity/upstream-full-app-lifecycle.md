# Flutter application lifecycle binding

Pinned `services/lifecycle_test.dart:11, :22, :33` pass. An
embedder can read the preexisting native state before any lifecycle
packet, including paused. The binding keeps all five exact state
names in state-machine order. A stale initial state never overrides
a state already observed from the host.

Pinned `widgets/app_lifecycle_listener_test.dart:91, :124` now pair
through a persistent observer of actual flutter/lifecycle string
payloads. The adapter synthesizes resumed before an initial
inactive message when starting detached, and all of hide, pause,
restart, show and reversible detach on later jumps. The callback
sequence matches the pinned original for all tested transitions.
A fresh binding accepts its first state without inventing earlier
states, and duplicate messages do not notify again.

Six native lifecycle tests passed. This is an opt-in embedder binding;
the portable Runtime's separate process shutdown model is unchanged.
