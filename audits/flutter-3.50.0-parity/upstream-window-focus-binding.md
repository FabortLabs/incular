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

## Focus entry into a retained view

Pinned `widgets/view_test.dart:516,567,622` passed. Native structured
view-focus events used to change only window appearance, not route
keyboard focus. All four native baseline cases reproduced the gap.
A structured exit now parks focus outside the widget subtree; forward
and backward entry select the corresponding edge of its Tab order.
Undefined entry preserves the latest widget request or restores the
same eligible parked node, without selecting its text again.
Ordinary OS chrome activation remains separate from view traversal.

Directional selection waits for measured controls, including on the
first frame. Request order also survives a node attaching late:
an older request does not override the host's direction, but a newer
eligible widget request does. The latter order is pinned in an
authored Flutter probe. All 24 focused native focus, window and
routing tests passed. No automatic native window raise or nested
engine ViewAnchor focus request is claimed.

The pinned authored editor probe and nine focused native checks also
verify a reverse selection (base 4, extent 1). Exiting via undefined
view focus retires the old native text-input client. Undefined
re-entry restores this editor and the same directional selection
using a fresh client. This is an additional authored scenario, not
another original registration.
