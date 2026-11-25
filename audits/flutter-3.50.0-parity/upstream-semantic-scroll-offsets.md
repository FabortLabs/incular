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
