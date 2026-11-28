# InteractiveViewer pointer signals

## Mouse wheel and trackpad scroll

Six selected originals from `widgets/interactive_viewer_test.dart`
verify mouse-wheel zoom and its disabled state, callback coordinate
spaces and zero end velocity, default discrete trackpad panning,
opt-in discrete trackpad zoom, and opt-in pan/zoom stream scaling.

The retained runtime tests replay engine packets with a 2× device
pixel ratio and assert the logical event, originating device, focal
point and controller transform. After the mouse reaches 2.5× at
translation (-150, -150), a trackpad scroll of (100, -25) retains
scale and moves the scene to (-250, -125). Horizontal-only default
trackpad scroll pans; horizontal-only opt-in zoom does nothing.
The opt-in stream returns 1.4993025 for -81px vertical pan and
does not add another scale increment when only its horizontal
component changes. An opt-in discrete -138px scroll yields
1.9937155 at approximately (-99.37155, -99.37155).

Previously the retained route labeled every discrete engine
scroll a mouse wheel, and InteractiveViewer would drop a
trackpad packet if it retained that identity. The new route
preserves the pointer kind and event time. Trackpad translation
uses the locally projected scroll vector, without changing the
global scroll read by listeners or the Shift behavior for mice.
Focused packet and independent signal-projection controls passed.
These are framework input replays, not live trackpad hardware.

## Discrete scale and anchored wheel zoom

Three more pinned originals pass: :1579 discrete pointer scale,
:874 equal forward/backward mouse scroll, and :906 reading the
scene point from interaction callbacks. Native runtime replay
red-tested the previously dropped scale signal (1.0 instead of
1.5), then confirmed three sequential factors of 1.5 accumulate
to 1.5, 2.25, and the configured cap of 2.5. Invalid factors
cannot alter the transform. Discrete scale is delivered to the
pointer-signal listeners directly; it is not a wheel event or a
synthetic pan/zoom stream.

Equal positive and negative wheel deltas restore the starting
scale in the wide-range scene. An off-center wheel callback at
local (80,80) gives the matching global location and the
transform keeps scene point (80,80) under that same pointer.

## Finite and unbounded touch boundaries

Twelve pinned originals in :29–532 pair with mounted touch
packet traces: the exact-fit boundary, a 10px margin, no boundary
and the nine free/aligned/horizontal/vertical pan scenarios.
An all-infinite margin now means no rectangle is used; a pan
does not try to construct an impossible infinite Size. Both
unbounded native probes previously panicked at that boundary.

The no-boundary viewer pans in both directions and pinches down
to 0.8. A flush child cannot pan until enlarged. A 10px margin
stops a 20px drag at (-10,-10) and limits the pinch-out scene
size to the 200/220 scale. The aligned 45-degree drag selects
vertical, and explicit horizontal/vertical axes reject the
opposite motion. The prior scroll and zoom anchor cases remain
green in the same native target.
