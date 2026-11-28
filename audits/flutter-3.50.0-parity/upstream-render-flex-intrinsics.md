# RenderFlex dry geometry and intrinsic sizes

## Main and cross-axis intrinsic sizing

Five pinned originals in `rendering/flex_test.dart:979–1095` passed.
The native `flutter350_render_flex_intrinsics` target mounts
real Flex and Paragraph render boxes. It verifies aspect-ratio
contributions of 300, 500 and 700; two 16px gaps only on the main
axis; and the 10px/20px/30px text reflow breakpoints in both
factor orders, including the 94px/95px boundary. Both minimum and
maximum intrinsic queries are checked. The paragraph tests use
the pinned FlutterTest font extracted by
`extract-flutter-test-font.py` and read from
`INCULAR_FLUTTER_TEST_FONT_PATH`.

Dry and retained flex now share the two-pass child allocation:
reserve actual inflexible contributions and gaps, then divide
remaining space among flexible children. Main-axis intrinsic
queries ignore tight fit and use the largest per-flex contribution
times the total factors. Cross-axis intrinsic queries measure each
child at its own allocated main extent, not the line's full width.

The additional mounted dry-size/baseline probe covers horizontal
baseline alignment with a child without a baseline, reversed
horizontal cross alignment, and bottom-up columns. It confirms
that speculative queries do not change previously laid out sizes
or baselines. These controls do not by themselves count the later
upstream custom grid-box baseline tests. Standard flex layout and
baseline regression targets also passed.

## Reflowing baseline-aligned children

The three pinned originals at :1133, :1167 and :1200 also passed.
Native fixtures replace the custom RenderFlowBaselineTestBox with
real 10px grid paragraphs and 0px top/bottom baseline markers.
An unbaselined grid wraps 10px boxes without any text baseline.
At 200px and 300px, the tests compare Flutter's exact dry sizes
and baselines, cross intrinsic heights, and the corresponding
native retained-layout measurements. A child with no baseline
reserves its height but cannot move its siblings' common baseline.
