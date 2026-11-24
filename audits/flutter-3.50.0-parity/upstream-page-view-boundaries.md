# PageView layout options, controller handoff and pre-layout commands

All eleven pinned originals selected here pass. Rust boundary target passes 3/3; controller-edges target passes five and intentionally fails one.

## Boundary geometry

- :1173 partial: the same retained viewport reports default HardEdge and updated AntiAlias, and its recorded rectangular clip disappears when disabled. Rust diagnostics plus display list recording do not reproduce the reference painting-context mode callback or certify rectangular antialiasing pixels.
- :1207 partial: the default half-fraction page starts at x=200 with end insets, then x=0 when padEnds(false) is repumped. This is actual geometry corresponding to Dart's inspected SliverFillViewport property.
- :1223 partial: the nearest f32 width to 392.72727272727275 allows initial page 152 and a jump to final page 365 of 366 without exceptions. This does not certify the exact f64 arithmetic path.
- :1304 reference only. Flutter's controller exposes explicit onAttach/onDetach hooks; no corresponding public Incular hook was found.

## Retained controllers

- :1384 and :1413 match. An added controller can drive pages 0, 1, 2; rebuilding without an external controller is safe and shows page 0 when no prior selection was made.
- :1392 and :1404 partially match. A replaced/removed controller cannot move the live view; the replacement can drive it. Dart also asserts on commands sent to the detached controller, whereas Rust can defer commands. These rows compare actual visible authority, not identical API errors or the selection transfer tested separately at :591.
- :1572 partial mismatch. Flutter's first child build reads a null page while content dimensions are unavailable, then sees a number after rebuild. Incular's child builder already reads Some(0) on the first invocation and still reads Some(0) after rebuilding. Rust schedules an equivalent descriptor rebuild programmatically instead of tapping the fixture button.
- :1624 and :1650 partial match. A request issued after constructing the view but before its first runtime frame completes without throwing, then displays page 1, for both animate/jump. Flutter tests a covered Navigator page; the Rust fixtures test the command timing directly and do not claim equivalent Navigator layout suppression.

Run cargo test --locked -p incular-runtime --test flutter350_upstream_page_boundary_layout and cargo test --locked -p incular-runtime --test flutter350_upstream_page_controller_edges.
