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
