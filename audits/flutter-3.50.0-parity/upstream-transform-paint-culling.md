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
