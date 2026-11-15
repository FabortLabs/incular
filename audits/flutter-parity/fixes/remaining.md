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
