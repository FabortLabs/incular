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
