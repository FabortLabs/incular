# RenderWrap dry intrinsic sizing and overflow clips

Five pinned `rendering/wrap_test.dart` originals ran and passed:

- :33 and :112: fixed minimum 80×80 children with spacing and run
  spacing 5 return cross sizes 165, 80, 250, and 250 at main limits
  245, 250, 80, and 79, in both directions. Both public intrinsic
  bounds and dry size agree. A native unbounded control fits one run.
- :143 and :163: a single 80×80 child returns cross extent 80 at
  limits 100, 79, and 80, with no trailing run gap.
- :183: for a 100×100 wrap with a 200×200 preferred child, the
  explicit clip modes produce the corresponding retained clip,
  while omitted and none produce no clip. This compares actual
  display-list commands rather than a GPU screenshot.

A truly childless ConstrainedBox now sizes to its enforced minimum,
including when the parent's width 79 overrides the child's 80.
The dry query does not update the child's or wrapper's measured
geometry. The parallel-axis controls take the largest child minimum
or sum of maxima without gaps. Unsupported dry render boxes return
no estimate; in particular, the two pinned wrapping-paragraph dry
tests at :64 and :86 still need a non-mutating text sizing path.

All 32 focused native wrap, clipping and adjacent proxy cases passed.
The widget inspector build with the devtools feature also checks.
