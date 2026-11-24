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
