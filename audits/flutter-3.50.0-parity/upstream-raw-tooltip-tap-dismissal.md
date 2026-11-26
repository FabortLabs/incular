# RawTooltip tap dismissal

Pinned Flutter 3.50.0 `widgets/raw_tooltip_test.dart` registrations
640, 659 and 2368 passed. Native mounted tests dispatch real touch and
mouse coordinates. Tapping an interactive bubble dismisses it when
enabled, opt-out leaves it open, and a touch tap on a long-press
anchor or elsewhere dismisses an already-hovered tooltip. A separate
native control cancels a pending cold hover before it can open; this
authored case does not increase upstream coverage.

The window pointer route uses the same hit-path listeners as ordinary
input. A down on the tooltip's trigger stays in the gesture arena;
an unrelated down cancels the pending or shown tooltip without
consuming the target's input. No ancestor TapRegionSurface is needed.
The reverse starts at the last value actually sampled, so dismissing
a just-opened tooltip does not advance its forward animation for an
extra frame. A tooltip whose animation reached the endpoint but has
not yet settled keeps the current curve direction.

All 44 focused runtime tests passed, including animation, Escape,
hover, and ordinary tap-region route and mouse-button handling.
