# Current-route TapRegion, AspectRatio, and long-press routing

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

- TapRegion current-route registration (:1133) passes against
  Flutter and differs in `flutter_tap_region_route_parity`.
  After push, Incular calls both pages; Flutter calls only the
  current route. After pop, both call just the original page.
  The old page's element was retained under the second route, and
  the popped element was gone. Counts are after each transition
  settles; no claim about callbacks during transitions, outside-up
  or consumption.
- AspectRatio (:26, :37) matches finite loose constraints and
  unbounded width in horizontal scrolling: 500×250, 250×500,
  and 1200×600 respectively. Zero area (:54) also matches for a
  centered Placeholder with equivalent tight zero constraints
  rather than a native platform view resize; partial.
  See `flutter_aspect_ratio_geometry_parity`.
- All five original long-press Flutter registrations pass.
  `flutter350_upstream_long_press` drives real Runtime events and
  an injected frame clock. At :61, activation is at 500 ms, not
  499; at :106, movement before acceptance cancels; at :365,
  movement after acceptance reaches the move and end callbacks.
  These three are partial because the direct recognizer's
  add/close checkpoints are not independently ported. At :455,
  changing buttons after acceptance does not cancel either.
  At :427 a mask-only move at 300 ms must cancel before
  activation. Incular misses Cancel and fires Start and End;
  the next valid primary press still works. The new mask reaches
  Runtime input but is dropped on the ordinary recognizer move path.
