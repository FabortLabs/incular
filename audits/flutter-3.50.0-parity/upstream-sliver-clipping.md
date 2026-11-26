# Sliver paint bounds and clipping

Flutter's `RenderSliver.getMaxPaintRect` computes the full paint rectangle
relative to the current physical paint origin. Reverse axes reflect it
across the *current paint extent*, not the viewport or total scroll extent.
Pinning caps the logical leading shift at the scroll extent minus the
obstruction, and an infinite maximum ends at the live cache boundary.

Incular now shares this coordinate calculation with DecoratedSliver.
Two focused unit tests cover reverse orientation, reverse growth, a
reported cross-axis cap, a pinned obstruction, and an infinite paint
extent. All 12 decorated-sliver and painter-lifecycle controls still pass.

Five original SliverClipRect registrations ran on the pinned Flutter
checkout: custom bounds must not expand at small overlaps, followEdge
rejects overlapped hits in vertical and reversed horizontal viewports,
Clip.none allows overlap hits, and a leading clip stays pinned until its
inside extent is consumed. The complete paired overlap hit-test matrix is documented below.

## Retained rectangle and rounded-rectangle clipping

The public SliverClipRect and SliverClipRRect now clip the sliver, not
each box independently. They use hardEdge and antiAlias respectively,
and default to followEdge; Clip.none removes the layer and hit restriction.
A saved-layer rounded clip over five actual lazy rows produces one
compositor isolation, and its rounded corner rejects pointer hits.

The pinned rectangular registrations at :70, :534, and :548 pair with
the native command and hit tests. A half-height clip accepts y=25 and
rejects y=75; removing it preserves the mounted child. A top inset
stays fixed until its inside span is consumed (scroll 0/100/385
gives top 30/0/−15).

Three rounded preserve-shape references (:684, :720, :751) pair for
vertical, horizontal and reversed vertical scrolling: 40 px corners
are preserved with an 80 px final main extent even under a 50 px
header overlap. The focused native clip tests, and the adjacent native
decorator, grouped focus and grouped tap tests passed.

Elliptical radii and the rest of the nested and overlap edge cases
remain open. No physical GPU pixel readback was required to compare
these exact retained clip commands and input targets.

## Overlapping-header hit matrix

Both dynamic registrations now pair: rectangle :384 and rounded
rectangle :489. Each runs all seven axis, reverse, and overlap-mode
variants. At a 50 px overlap, real hit testing at logical main
positions 25 and 75 agrees with pinned Flutter: none allows the
covered hit, and followEdge and preserveShape block it without
blocking the uncovered content. The native fixture makes the
overlapping header itself transparent to hit testing so it observes
the clip's decision rather than the header's paint order. Fourteen
expanded fixtures represent two upstream registration sites.

## Live clip changes

An authored pinned Flutter probe changes a notifier from top inset
50 to 30. Hit testing reflects the new top before the next paint,
and paint uses the same newly resolved clip without replacing the
child. Both Flutter and native obtain the revised custom shape
exactly once. The native test also verifies unchanged rebuild and
layout counts. The retained viewport subscribes only to clip sources
that currently participate in it, and drops subscriptions with the
layers.

A separate authored replacement probe returns false from shouldReclip.
The mounted clip preserves its top at 50, then a 10 px scroll
recomputes the new clipper's top (20 in paint coordinates). A second
replacement returning true immediately changes the top to 45. Paint,
hits, and retained child state are checked without conflating reclip
and layout. All seven focused clip cases passed.

## Approximate accessibility clipping

The pinned authored probe gives the sliver a 20 px paint inset and
a separate 50 px accessibility inset. The label entirely above
50 is omitted; a label spanning y=40–70 is exposed only at y=50–70.
A notifier brings the first label back and restores the crossing
label's original bounds and node identity, without changing the
paint clip. This works when semantics are updated before painting.
The same native test checks that it requires no rebuild or layout.

58 focused native tests passed across sliver clips, resizing headers,
and sliver semantics.

## Retained clip property changes

Pairs the pinned rectangle :143/:196/:431 and rounded rectangle
:155/:234/:291/:536 registrations. As the header begins to overlap,
the child keeps its 100 px size, but the visible clip shrinks by 50 px.
Changing the overlap policy restores the full clip; disabling clipping
removes the layer and makes the overlapped point hittable. Re-enabling
clipping does not replace the child or reset the scroll offset.

The rounded test also follows the physical radius as the configuration
changes from a 10 px top-start corner in LTR, to a 20 px top-right
corner, back to the logical radius, and then to RTL. The same child
stays mounted throughout. All ten focused native tests passed.

## Committed clip measurements

A lazy sliver's provisional 48 px box estimate no longer escapes to a
custom clipper. A 100 px child supplies exactly one 800 × 100 size in
both the pinned authored Flutter probe and the native retained viewport.
The viewport waits until its size/position sweeps settle before updating
its shared sliver clip layers; painted, pointer and accessibility
geometry still use the same selected clip frame. All 21 focused native
clip, decorated-sliver and grouped editor tests passed.

## Custom boundaries and rounded hit edges

Pairs rectangle :260/:457/:495 and rounded rectangle :73/:361. A
30 px custom inset is not expanded to a 10 px overlap; a subsequent
50 px overlap trims it to 50 instead. Physical half-width clipping
accepts x=25 and rejects x=75 in horizontal layout. In reversed
vertical scrolling the physical half-height clip accepts logical
main=75 and rejects main=25. A default antialiased rounded clip
includes the center and (25,25), but excludes its (0,0) corner.

All 14 focused native clip tests passed.

## Nested rectangular and rounded scopes

The pinned authored and native fixtures combine a half-width outer
clip and a rounded inner saved-layer clip over five actual rows.
The right-side tap is blocked by the outer clip, and the top-left
tap is blocked by the rounded corner. Removing the outer clip
admits the right-side tap but keeps the inner boundary and row
identity. Native painting records one layer for each scope, not
ten per-row clips. All 15 focused native clip tests passed.
