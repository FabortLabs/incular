# Local animation listener lifecycle

Both exact pinned Flutter 3.50.0 registrations
`animation/listener_helpers_test.dart:9,17` passed. Removing an
unregistered status or value callback from `LazyAnimationListeners`
causes no start and no stop of parent observation. The Rust methods also
report that nothing was removed.

Three authored reference cases check the separate value and status
duplicate policies, removal during notification, and callback failure
isolation. Five focused native tests cover the same behavior. The first
registration of either kind starts observation; only the last real
removal stops it. Duplicate value callbacks notify once by identity,
whereas status callbacks notify for each registration. An absent
callback neither detaches a live parent nor underflows a counter.
The local dispatch snapshot skips removed identities, delays newly
registered callbacks, and returns original caught panic payloads to the
caller without preventing later delivery in unwind builds.

The helper is for custom lazy animation objects. This does not change
the eager listener policy of `AnimationController` or claim that every
existing derived animation graph now uses the helper.
