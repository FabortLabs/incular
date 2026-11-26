# Physical accessibility actions for drag gestures

Pinned `rendering/proxy_box_test.dart:100` starts with a tap
and horizontal drag handler. Flutter advertises tap, ScrollLeft
and ScrollRight; narrowing the allowlist to tap and ScrollLeft
removes only ScrollRight. Before the repair Incular advertised
only tap. The retained Rust regression checks the exact action
lists and stable semantic identity across the allowlist update.

An authored Flutter probe and an AccessKit-to-Runtime test
exercise all four directions on a 200×100 detector translated
by (20,30). The default 0.8 factor produces deltas (-160,0),
(160,0), (0,-80) and (0,80), and the transformed center is
(120,80). As in Flutter, primary delta matches the signed
axis; no source timestamp is invented, and localPosition
defaults to globalPosition for these synthetic events. Flutter
uses a null pointer device; Incular reports its Unknown kind.

Changing the factor to 0.5 and retaining only tap, left and
down yields -100 and 50 on the allowed axes. A stale native
request for right is rejected by the live descriptor. Separate
Rust controls confirm that drag-only gestures are accessible,
pointer blockers remove the arrows, and excluding gesture
semantics does not disable a physical tap.

Native projection now maps explicit directions separately.
Existing logical forward/backward actions on scrollable views
remain supported and their legacy projection tests pass.
These probes use the actual portable graph and native data
adapter, not a live OS screen reader.

## GestureDetector recognizer phases

Pinned `widgets/gesture_detector_semantics_test.dart` :13 and
:62 use start-only recognizers. A vertical recognizer advertises
and receives only up/down; a horizontal one only left/right.
Previously Incular needed an update callback even to expose
the action, so neither start-only recognizer was reachable.
:111 registers an axis recognizer and a pan; both are invoked.

The authored Flutter and Rust fixtures check the full four
phases for each installed recognizer, axis first and pan
second. Down/start report the true local and transformed
global centers. Update retains the synthetic delta and the
globalPosition fallback for its localPosition. End reports the
center plus delta transformed into world space, zero velocity,
and zero primary velocity for the axis versus no primary
velocity for pan. Cancellation is not synthesized.

## Explicit Semantics callbacks

A pinned Flutter widget probe installs onScrollLeft, onScrollRight,
onScrollUp and onScrollDown together, then executes all four in
turn. Before the Rust fix these methods shared two callback slots,
so the later vertical registrations overwrote the horizontal ones.
The mounted Rust counterpart now advertises four physical actions
and invokes left, right, up and down independently. Builder-only
annotations expose their action even without a label.

The retained AccessKit control removes the left callback while
logical backward and physical down stay installed. The stable
native ID no longer accepts the old left request; it cannot fall
through to the unrelated backward callback. Genuine scroll
containers still retain their logical fallback, and callers can
register logical forward/backward callbacks explicitly.

Pinned `widgets/semantics_test.dart:474` passes on Flutter. This
is partial for that registration: it also covers dismiss, clipboard,
cursor movement and additional actions beyond this directional batch.

## Full ordinary Semantics action inventory

Pinned `widgets/semantics_test.dart:474` is now fully paired. An ordinary
mounted Incular annotation advertises the same 21 public callback
actions on one leaf. Every action reaches exactly its own callback,
including both character directions with selection extension and
the selection/text payloads. Its declarative focus callback does not
steal runtime keyboard focus. This comparison is to the ordinary
widget, not the separate SliverSemantics fixture. One targeted Rust
case and the pinned original passed.
