# RawScrollbar track and thumb painting

Four pinned `widgets/scrollbar_test.dart` registrations passed.
The native focused target exercises the corresponding raw geometry
and the retained overlay's display-list recording:

- :89 keeps a 3.5 px minimum thumb on a 100,000 px range, at
  the boundary and then one pixel into the range.
- :122 preserves thumb length at six normal scroll offsets and
  moves it proportionally to the available travel.
- :247 checks all four explicitly selected edges and exact 2.5 px
  thickness. The thumb uses the metrics' 23 px viewport extent when
  the canvas is wider; the canvas sets the physical cross edge.
- :1682 paints a 600 px track above retained child content and
  draws the thumb at y=10 with length 348. The track is not inset
  by the thumb margin. A second paint moves the thumb to y=242
  after the controller reaches the opposite end.

The explicit retained overlay was not recording its foreground.
It now has a foreground picture and rerenders only when tracked
geometry or opacity changes. The test makes the track color
visible to inspect its rectangle; a transparent track still draws
no pixels. This is not GPU readback. Default orientation, hover,
and routing pointer input into the explicit overlay are separate.

All 24 focused native scrollbar and retained scrolling tests passed.

## Retained thumb drag, hover, and track tap

Three exact registrations in the pinned `widgets/scrollbar_test.dart`
passed. The native `flutter350_raw_scrollbar_input` target drives
real pointer events through Runtime with the automatic overlay disabled:

- :622: tapping below and above the thumb moves the controller
  between 0 and 400; the 360 px thumb lands at y=240 and y=0.
- :823: hovering at (794, 5) reveals the 90 px thumb without a
  prior scroll, using the initial layout's viewport metrics.
- :855: dragging the 90 px thumb 10 px with touch moves the
  controller over 20 px, paints the thumb at y=10, and closes
  the activity on release.

The track tap clamps its target, drives a 100 ms ease-in-out
animation, and pages by 80% of the viewport (480 px for a 600 px
view with enough content). A move past touch slop or a cancel
before a quick tap commits does not page. Flutter can also fire
its TapDown callback after a held press's deadline; that path
and overscroll during thumb drag are not covered by these tests.
Incular uses an explicit shared controller for these comparisons;
Flutter's automatic primary-controller wiring is a separate API.

All 13 focused native pointer, paint, and editor-focus tests passed.
