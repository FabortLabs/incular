# Align, Padding and fractional boxes forward real baselines

The pinned Flutter originals and three supplemental geometry probes
pass. Before the fix, each retained Incular wrapper placed its child
correctly but reported no baseline, so wrapping text in otherwise
pure layout widgets changed baseline alignment higher in the tree.

| Wrapper | Measured result in both engines |
| --- | --- |
| Align | A 100×100 child carrying a 40px baseline inside a 200×200 box reports 40, 90 and 140px at top-left, center and bottom-right. A retained update to a baselineless child clears it without changing placement. |
| Padding | Insets (7,13,15,11) yield a 122×124 wrapper, place the child at (7,13), and expose the 53px baseline. Removing the metric keeps the geometry and reports no baseline. |
| FractionallySizedBox | Factors of 0.5 under a 200×200 maximum and a 160px minimum height constrain the child to 100×100, place it at (0,30), and expose the 70px baseline. |

The first comparison of `rendering/shifted_box_test.dart`
:57, :71, :84, :183 and :200 covered only the laid-out metrics
and their child offsets. The direct query is covered below.

The separate `rendering/positioned_box_test.dart` cases :13 and :25
assert only the actual box sizes. Their Flutter originals and retained
Rust checks agree on 200×200 when factors are absent and on
100×200 → 200×100 → 100×100 when changing the specified axes. These
are full behavioral comparisons, even though the Rust tests use its
public Widget façade. :50 also verifies actual layout at
100×0 → 50×50 → 200×200. Its direct intrinsic methods were paired
in the follow-up below.


## Align intrinsic factors

The pinned positioned box at :50 now pairs. All four dry intrinsic
queries update across the same mounted render box and match
100×0 → 50×50 → 100×100. Without a factor the real bounded box
still expands to 200×200, not its intrinsic size.

Two additional reference probes compare a child whose own intrinsic
extent differs from its laid-out size. Align forwards the original
cross-axis input, then applies its factor only to the child's answer.
Mounted IntrinsicWidth formerly chose 27.5 px from the actual child
instead of 50 px from its reported size; it now pairs with Flutter,
as does the 100 px height case. These probes add no original count.
All 26 selected native tests passed.

## Dynamic intrinsic dependents

Two pinned `rendering/dynamic_intrinsics_test.dart` cases pair at
:81 and :95. Growing a leaf from 100×100 to 200×200 recomputes the
existing intrinsic root and updates all four unbounded intrinsic
queries on the intermediate parent. The Rust test updates only
the already-mounted leaf; it does not replace the ancestor or
change the frame constraints. An unfactored Align provides the
intermediate forwarding box. This checks dependency propagation
and sizes, not Flutter's custom RenderBox subclass or the debug
error for a negative intrinsic dimension.

## Paint and hit-test proxy intrinsics

Five authored Flutter controls cover ColoredBox, zero Opacity,
ClipRect, IgnorePointer, and RepaintBoundary. All four queries
report the child's natural minimum (55 × 110) and its separate
preferred maximum (100 × 200). The ordinary box still measures
55 × 110; IntrinsicWidth sizes it to 100 × 110, and IntrinsicHeight
sizes it to 55 × 200.

Before the fix, the native dry query returned no size even through
a color, and a zero-opacity child reported only its ordinary 55 px
width. Dry queries and layout now use the same list of forwarding
proxies so a wrapper that only paints or handles input cannot
substitute real layout for the child's intrinsic result. All 22
selected native cases passed. These authored comparisons do not
increase the pinned upstream registration count.

## Direct dry alphabetic baselines

Twelve exact pinned registrations now pair: :45, :57, :71, :84,
:120, :138, :152, :167, :183, :200, :212 and :228.
The native queries use new constraints without running a live
layout. The equivalent leaf is 50 × 50 with an explicit 40 px
alphabetic baseline. Top-left, center, and bottom-right alignment
report 40, 115, and 190 px in 200 × 200 bounds. A baselineless
child returns no result. Finite factors, unbounded, 1 px and zero
bounds match. Overflow and fractional wrappers use the child
constraints, Padding adds 10 px, and Baseline changes the result
to 30 px. A native control verifies negative offsets and absence
of a baseline when the child has none.

The query preserves both the previous measured baseline and the
previous laid-out bounds, and it does not increment live layout
counts. It shares the text-shaping helper with the dry size query.
This is the alphabetic path; the ideographic query and Flutter's
literally childless positioned box remain separate. All 23 focused
native baseline and intrinsic tests passed.

## Childless aligned boxes

The exact :14 and :34 pinned registrations now pair. An actual
Align with no retained child returns no baseline for both the
default center and top-left alignment. It still fills each
unfactored bounded axis: at a maximum of 100 × 100 it measures
100 × 100, and adding a height factor gives 100 × 0. The same
childless shape is preserved by Center and the alignment transition
descriptor. All 24 selected baseline, layout, structure, and
transition tests passed. A separate sliver-alpha fixture was
also checked and reproduced its pre-existing failure unchanged
on the prior commit; it is not evidence for this slice.

## Typed dry baseline queries

The final original in `rendering/shifted_box_test.dart` (:98)
passed. With the same top-left fixture, the public typed query
returns 40 px for both alphabetic and ideographic requests without
running live layout. Flutter's actual RenderParagraph uses its
alphabetic metric for both policies at this revision; supported
retained paragraph wrappers apply the requested placement to that
same metric. This is not a claim about a custom render box that
supplies independent metrics. All 10 native baseline tests passed.
