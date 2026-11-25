# FocusManager application lifecycle

The pinned Flutter 3.50.0 originals in
`widgets/focus_manager_test.dart:358,393,420,448` passed in nine
native-runner invocations. Rust drives the retained runtime through
`PlatformEvent::Lifecycle` and checks each intermediate state, not
just the outcome after resume.

- Android and iOS retain focus across suspension and resume. The desktop
  runtime withdraws primary focus as soon as the app is inactive or
  suspended and restores the same live target on resume.
- Detaching the parked node or replacing it on the same retained
  element prevents it from passing focus to the new node.
- A request from another node after parking wins even before any frame
  drains it. The original node's observers see no transient refocus.
- Refocusing an editor on resume preserves its selection. A subsequent
  desktop Tab to a different editor still selects all by default.

Desktop blur and the pending-request branch failed on the baseline.
All 22 focused Rust focus/selection/platform tests passed after the
repair. These are application lifecycle events; native window focus,
browser integration and lifecycle observer transition synthesis are
separate from this comparison.
