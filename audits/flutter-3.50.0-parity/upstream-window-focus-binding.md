# Platform view focus delivery

The pinned `services/binding_test.dart:195` registration passes. Its
platform dispatcher delivers a focused, forward-direction event for
view 0 to the services binding without changing the event.

The native owner is the window event router. `WindowEvent::focus`
retains the original window identity, direction, and focus state.
`Application::observe_window_focus` receives them after the matching
window's retained environment is updated. One window builds with
focus while the other does not rebuild. A later backward focus for
the second window is not attributed to the first. A normal OS focus
notification that has no directional data uses `Undefined`.

The second mounted test confirms that a reused window slot cannot
receive a previous generation's event, and a dropped subscription
stops receiving notifications. These checks do not assert Flutter's
separate view-level focus traversal policy or synthesize native OS
focus events.

Two direct mounted tests and eight adjacent runtime tests passed.
