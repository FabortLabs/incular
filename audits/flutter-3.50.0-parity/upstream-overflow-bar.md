# OverflowBar dimensions, placement and public controls

All nine named originals in widgets/overflow_bar_test.dart passed with the pinned Flutter 3.50.0 checkout. Incular widget-tree geometry uses keyed children of the same explicit dimensions. The focused Rust test target has three passing controls and four intentional mismatches.

| Registration | Result |
| --- | --- |
| :9 defaults | Reference only for the direct property assertions. The corresponding Rust builder defaults are private; alignment, overflow direction and explicit text-direction controls do not exist on the descriptor. Empty children and spacing are exercised in the behavior tests below. |
| :19 empty | Matches: tight 16x24 remains 16x24; loose constraints yield 0x0. |
| :46 horizontal | RTL mirroring and 0/10 gaps match for all four samples. The 48- and 32-high children stay top-aligned at y=0 instead of centered at y=8 and y=16 alongside the 64-high child. |
| :102 vertical | Partial mismatch for all six public downward configurations. At max width 100 Flutter uses one child per row and start/end/center horizontally; Incular puts the last two children in one horizontal run. At LTR start and zero gap the third child is at (64,48) instead of (0,112). Incular has no upward overflow direction setter, so the two reversed cases remain reference-only within this registration. |
| :205 width | At available widths 800/150 Flutter reports 152/150; Incular reports 152/152. Reproduced through the translated widget composition and with a direct finite root constraint, ruling out the sizing wrapper as the only cause. |
| :236 height | At available widths 800/150 Flutter reports 75/166; Incular reports 75/133 in both Rust harnesses. The latter is the height of two runs with an 8px gap rather than three vertically stacked children with two gaps. |
| :267 wider than intrinsic | Matches 800x600 and x origins [0,60,130] (LTR) / [750,680,600] (RTL). |
| :303 explicit row alignment | Reference only; OverflowBar has no public main-axis alignment setter for the six Flutter Row comparisons. Replacing it with a Wrap/Row would not test the public descriptor. |
| :365 zero area | Matches with nonempty text children: no crash and 0x0. |

Source trace: the Rust OverflowBar converts to a horizontal Wrap and maps overflow alignment to per-run cross-axis alignment. The Rust fixtures use only the OverflowBar API. Run "cargo test --locked -p incular-widgets --test flutter350_overflow_bar_parity -- --nocapture".

## Current retained implementation

The table above records the original Wrap substitution, not current behavior.
All nine original Flutter registrations passed again in the pinned SDK.
All twelve native cases passed in `flutter350_overflow_bar_parity`, including
the original matrix and the three additional cases below.

| Registration | Verified now on Incular |
| --- | --- |
| :9, :19 | The builder and constructor agree on defaults; absent alignment shrink-wraps, start honors ambient direction, downward order is the default, and the empty bar uses the smallest constraints. |
| :46, :102 | A fitting row vertically centers the children in both directions. An overflowing row becomes a single column in all eight original start/end/center, LTR/RTL and up/down configurations. |
| :205, :236 | The `IntrinsicWidth` and `IntrinsicHeight` compositions reproduce width 152/150 and height 75/166. This does not establish generic Dart dry-layout or intrinsic API parity. |
| :267, :303 | A wider tight parent keeps the default attached to logical start; explicit alignment fills the width and matches the corresponding Row's child edges in both directions. |
| :365 | Text children at zero width and height still lay out without error. |

The additional pinned `overflow_bar_boundary_test.dart` passes three
counterparts: at width 150 it is still a row, at 149 it becomes an upward
column without replacing children; explicit RTL wins over inherited LTR;
`IntrinsicWidth` ignores expanding row alignment during measurement; and a
single space-between child stays at logical start. Those three custom cases
do not add upstream registration counts.

The adjacent existing Wrap baseline-registration failure (`wrap_test.dart:878`)
reproduced unchanged. It is a separate audit item, not a failure of OverflowBar.
