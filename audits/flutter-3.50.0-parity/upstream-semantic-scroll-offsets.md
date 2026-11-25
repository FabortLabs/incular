# Accessibility scroll offsets

Pinned `widgets/scrollable_semantics_test.dart` registrations :101,
:132, :164, and :210 all passed. Incular now publishes an absolute
scroll action for a viewport with a nonempty range and implicit
scrolling enabled. The framework consumes y for a vertical view
(456 from 123,456) and x for a horizontal view (123), as Flutter
does. Both original positive registrations are partial at this
checkpoint: their expected directional action accompanies the
absolute action, while Incular still publishes its former logical
forward/backward aliases only. The two absence registrations fully
match; a fitting view and a view with implicit scrolling disabled
neither advertise nor accept the absolute action.

The retained runtime receives these requests via the normal
semantic action dispatch. A test also sends the real AccessKit
`SetScrollOffset` request to the projected node, verifies omission
of a missing payload or a nonfinite coordinate, and checks that the
correct native axis reports exact scroll metrics. Reversed and
fractional values are Rust controls, not additional matched originals.
Twenty focused Rust tests passed, including the native projection,
custom directional callback and existing offscreen-cache controls.

## Directional actions and edge availability

The viewport now advertises Flutter's physical swipe directions and
withdraws the blocked direction at each edge. A normal vertical view
advances for Flutter ScrollUp; reverse swaps the actions, and a
horizontal view uses left/right instead. Direct semantic swipes move
80% of the viewport (480px vertical, 640px horizontal), including
when reversed. The pinned authored control invokes the actions in
all four orientations and confirms fractional absolute jumps.

AccessKit uses the direction of the viewport's motion, not the
opposite swipe of its contents. Its projection now presents only the
appropriate native arrows and translates those arrows back to the
correct Flutter action. Existing logical forward/backward aliases
remain available to Rust callers; no unrelated native arrows are
advertised. Standalone custom callbacks retain their previous meaning.

All four edge originals in `widgets/list_view_semantics_test.dart`
(:17, :55, :94, :133) compare the same 10 tiles and out-of-range
jump request of 1500, including the pinned test font's measured
84px horizontal item width, and the resulting action changes at the
settled edge. The two positive absolute-offset originals (:101,:132)
now pair their expected directional action as well. Five authored
Flutter checks and 33 targeted Rust tests passed.
