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

Dart RenderWrap formatting (:851,:858) and the clipBehavior
setter (:962) have no equivalent and are reference-only. All
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
