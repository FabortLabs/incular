# Wrap basic directions, child baseline, and unmatched diagnostics

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All seven remaining original registrations report passes on Flutter.
Both six-state Basic Wrap geometry sequences match exactly
(:20, :134), including the RTL-named test's later LTR/up states.
The empty centered Wrap (:251) measures zero on both.

The baseline case (:878) is partial against Flutter's Ahem test
font and is independently reproduced without comparing font sizes
between frameworks. Flutter's own direct text and Wrap-text
Baseline boxes have the same height. In Incular, the direct
test uses the text's measured baseline at 81.13069 with a
100-high text box, giving 193.86931 at target baseline 175.
Adding Wrap loses the baseline and leaves height 175. This is
layout propagation, not inherent font metric disagreement.
See `flutter350_wrap_constraints_parity` and
`wrap_child_baseline_reference_test.dart`.

Dart RenderWrap formatting (:851,:858) remains reference-only. The clipBehavior update at :962 is now paired. All
22 original registrations in this file are now disposed of
as paired, partial, or reference-only, not all matches.

## Current baseline behavior

Wrap now reports the highest actual child baseline, including its positioned
Y offset. The old single-text regression is green. A new paired authored case
reverses two runs with a 7px gap, verifies that the second logical child is
the highest visual baseline, and then replaces both with a baseline-less box
to check that no stale baseline remains. The original Flutter control and both
authored Flutter cases passed on the pinned SDK; 18 focused native tests
passed across the Wrap constraints and builder targets.

The original registration remains a partial comparison because its 100×100
FlutterTest glyph and fixed 200px outer height are font-specific. The native
test measures its own font and compares direct text with the wrapped result.

## Overflow clip and retained mode

Pinned Flutter cases `:787` and `:962` pass, as does the authored
mode-switch probe. Native display lists have no clip by default, even
if two 500×500 children overflow an 800×600 box. Hard-edge and both
anti-alias modes attach their requested rectangle only while overflowing.
Switching mode retains the same render object; replacing two children
with one drops the stale clip. This verifies paint commands and their
coverage modes, not a GPU anti-aliasing pixel comparison.

## Exact font and dry baseline follow-up

The original at :878 now matches exactly using the extracted pinned
FlutterTest font: 100×100 text, first baseline at 75, and 100×200
Baseline parent at target 175. The native checks also confirm dry
queries do not lose a Wrap's baseline before layout. Both new
native baseline assertions reproduced a missing baseline before
the fix and are green afterwards.

An authored probe separates a pinned Wrap bug from the ordinary
case. For two runs (40px and 20px, gap 7), down direction agrees
at 30 in both frameworks. In up direction, pinned Flutter reports
actual baseline 15 and dry baseline 82, counting the 67px content
height as free space. Querying with live constraints can trigger
Flutter's dry-vs-actual cache assertion. Incular uses the real
positioning for the dry query and reports 15 in both paths. The
ordinary original is paired; this authored difference is not
counted as parity.
