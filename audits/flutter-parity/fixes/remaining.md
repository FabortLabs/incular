# Remaining findings pass

Second pass over the findings the first fix pass left blocked, deferred,
out of scope or invalid (switchers-004). One commit per finding.

## switchers-004: fixed

`AnimatedSize::reverse_duration` stores Flutter's `reverseDuration` and
forwards it to the size controller at creation and on every update
(`didUpdateWidget`), as `widgets/animated_size.dart:74-78` does. Flutter
3.41.7's `RenderAnimatedSize` only runs forward
(`rendering/animated_size.dart:309-374`), so the value does not change
any size run; the regression test checks that a shrink still takes
`duration`.

## gestures-013: fixed

`trackpadScrollCausesScale` was done in the first pass; this pass adds
Flutter's `DeviceGestureSettings` (`gestures/gesture_settings.dart`).

- `incular_core::DeviceGestureSettings { touch_slop }`, with `pan_slop()`
  (twice the touch slop) and `from_physical(slop, dpr)` for
  `DeviceGestureSettings.fromView`. Re-exported from `incular-config` and
  `incular-gestures`.
- `incular-gestures` has one settings-aware `compute_hit_slop(kind,
  settings)` and `compute_pan_slop(kind, settings)`, as in
  `gestures/events.dart`, replacing three private copies. Mice keep the
  precise-pointer slops; `compute_scale_slop` has no override, as in
  Flutter.
- Every slop-using recognizer takes the settings: the pointer, scale,
  tap-and-drag, multi-tap, multi-drag and force-press recognizers.
- `RuntimeEnvironment::gesture_settings` is Flutter's
  `MediaQueryData.gestureSettings`. The retained tree hands the nearest
  `MediaQuery`'s settings (or the window's) to the recognizers it creates
  for gesture detectors, scale detectors and scroll drags, as
  `GestureDetector` and `Scrollable` do with
  `MediaQuery.maybeGestureSettingsOf`.
- Hosts: Flutter's desktop and web embedders report no touch slop, so the
  desktop and web hosts leave the default. No Incular host runs on Android
  or iOS yet; a mobile host fills the value with `from_physical`.

Tests: `incular-gestures/tests/device_gesture_settings.rs` (5) and
`incular-widgets/tests/gesture_settings.rs` (4). `incular-gestures` tests
and the related widget test files pass; harness `gestures__` 11/11.

## navigation-006: fixed

The Router's `BackButtonDispatcher` (`app_shell/router.rs`) now has
Flutter's child dispatchers (`widgets/router.dart`):

- `create_child` returns a `ChildBackButtonDispatcher`. Its parent consults
  it only after `take_priority`, which calls the parent's `defer_to`.
- `dispatch_back` (`invokeCallback`) offers the request to the children
  that took priority, newest first, and then to the dispatcher's own
  callbacks.
- `BackButtonDispatcher::take_priority` clears the children, and `forget`
  removes one. Dropping a child's last callback subscription forgets the
  child, as `ChildBackButtonDispatcher.removeCallback` does.
- `defer_to` ignores a dispatcher that is not its own child, so priority
  cannot form a cycle.
- `RouterConfig::with_back_button_dispatcher` takes
  `impl Into<BackButtonDispatcher>`, so the root router passes a
  `RootBackButtonDispatcher` and a nested router passes a child.
  `RouterConfig::back_button_dispatcher` now returns the general
  dispatcher type (API change).

The navigation-layer dispatcher used by `PopScope` and `BackButtonListener`
(`navigation/back_dispatch.rs`) already had children and is unchanged.

Tests: `incular-widgets/tests/router_back_priority.rs` (8), including two
nested `Router`s. `app_shell`, `completed_module_integration` and
`incular/tests/public_api` pass.

## navigation-011: fixed

Flutter's `TransitionDelegate` for declarative page updates
(`widgets/navigator.dart:1016-1260`), in `incular-navigation`'s
`transition_delegate.rs`:

- **Records.** `Navigator::set_pages` describes each update as a
  `PageTransition`. It holds one `PageRouteRecord` per page of the new
  history, plus one `ExitingPageRoute` per removed route, with its
  `location`: the kept route it sat above (`locationToExitingPageRoute`).
- **Decisions.** `PageEntering::{Push, Add}` and
  `PageExiting::{Pop, Complete, Remove}`, with `mark_for_*` methods.
- **Default.** `DefaultTransitionDelegate` follows Flutter: an added new top
  pushes and the other added routes are added; the last removed route above
  the new top pops and the other removed routes complete. Records arrive
  prefilled with these defaults, so a custom delegate changes only what it
  cares about. This is a Rust-first shape: Flutter asserts that every record
  is decided.
- **API.** `Navigator::set_transition_delegate` (closures implement the
  trait) and `clear_transition_delegate`. `take_page_transitions` hands
  pending decisions to the presenter; nothing is recorded until a presenter
  first reads, so a navigator without one keeps nothing.
- **Outlet.** `RouteOutlet::drive_route_animations` reads the decisions.
  An `Add` starts complete; a `Pop` animates out where the route was. A new
  `ExitPlacement` (`Bottom`, `Above(route)`, `Top`) replaces the implicit
  `below`/`until` encoding. `Complete` and `Remove` leave at once (Incular
  routes have no completion result, so the two differ only in what they
  record). Imperative push, pop and replace keep their transitions, as in
  Flutter.

**Behavior change.** A `set_pages` update that added several pages
previously animated all of them, and replacing the top page kept the old
page under the new one. Both now match Flutter's default.

**Tests:**
- `incular-navigation/tests/transition_delegate.rs` (10);
- four outlet tests in `incular-runtime/tests/runtime/route_outlet.rs`.

`incular-navigation` and `incular-runtime` tests pass. Harness
`navigation__` is 6/12, the same six documented divergences as before.
