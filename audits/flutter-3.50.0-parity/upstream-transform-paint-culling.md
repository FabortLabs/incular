# Non-drawable transforms

The five pinned `widgets/transform_test.dart` originals at
:310, :321, :332, :343 and :471 passed. In the native baseline,
NaN/infinite transforms could panic while an ancestor clip projected
the child's bounds, and a zero scale still emitted its child's
display list and ran the painter.

A non-finite or singular affine now short-circuits the paint traversal,
retained visual bounds, and compositor culling before projecting
invalid geometry. Ordinary placements use a cheap finite-translation
check, with no matrix inverse for ordinary painting. The widget fixture
checks NaN and both infinities under an ancestor clip, all three zero
scale combinations, 0.01×, and a 90° rotation with the exact nonzero
matrix. A live scale control verifies that paint invalidated under a
hidden transform stays pending and runs once when restored. The
direct layer fixture checks that even a previously cached picture is
not submitted while hidden, the debug and size queries stay safe, and
restoring its transform reuses the same attached picture.

All 40 focused native tests passed. The comparator uses submitted
child paint and layer identity, not an exact count of internal
framework layers: Incular deliberately retains the dormant layer
handles needed for reappearance.

## Accessibility geometry before paint

After the painting fix, a non-finite transform above a labelled,
rounded-clipped child could still panic earlier in semantics
collection. All semantic bounds and clip projections now treat
a non-finite world matrix as empty geometry. The native test runs
one retained tree through valid → NaN → valid → infinity → valid
and verifies a valid 100×80 semantic label on each return without
publishing an invalid rectangle. This includes a real rounded clip
and preserves the existing finite projection path.

The authored pinned `non_finite_transform_semantics_test.dart`
confirms that Flutter's debug semantics node reports an assertion
for the non-finite rectangle. Incular deliberately recovers instead;
this is recorded separately and adds no upstream paired count.
All 14 focused native transform and clip-hit tests passed.
