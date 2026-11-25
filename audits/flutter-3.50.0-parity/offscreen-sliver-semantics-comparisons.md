# Paired offscreen sliver accessibility and initial-offset checks

Official pinned Flutter `packages/flutter/test/widgets/sliver_semantics_test.dart:246`
ran successfully against `ab2912035e`. Production `c647cb85` reproduced
two distinct differences in the executable
`incular-runtime` target `flutter_sliver_semantics_parity`:

1. A provided initial scroll offset of 300 became 1212 on the first
   Incular layout. The viewport and rows are 200 px; this is not a
   float-rounding difference.
2. After both establish an actual 300 px offset, with verified 200 px
   viewport and equivalent first four row positions, Flutter shows
   Items 1/2, hides cached Items 0/3, and omits Item 4 onward. Incular
   exposes all 30 items with hidden=false. Flutter's default and
   forced-retention controls also passed with the same post-layout
   jump; only `SliverEnsureSemantics` can keep Item 4 in that case.

The second fixture normalizes initial positioning independently instead
of attributing the tree difference to the first-frame offset. These
are framework semantics-tree checks, not a native screen-reader run.
Relevant source: Flutter `rendering/viewport.dart:470–479,935–953`;
Incular `crates/incular-widgets/src/scrolling/layout.rs:78–97`,
`tree/semantics.rs:577–603,1302–1315`, `scrolling/sliver_semantics.rs:154–196`.
The offset difference is consistent with anchoring corrections during
estimate refinement (48→200 on six rows; +912); this is a source
hypothesis, not a verified fix or complete explanation.

## Literal initial offset repaired

The first retained layout has no previously painted row to anchor.
It now preserves the controller's literal initial offset while
refining estimated sizes rather than treating estimates as painted
positions. The paired Rust setup now establishes offset 300 on its
first 200px viewport frame, instead of drifting to 1212. Later
retained anchor corrections are not disabled. The literal offset
check, three sliver correction tests, 39 scrolling tests, and eight
centered viewport tests passed. The separate accessibility-tree
assertions remain open at this checkpoint.
