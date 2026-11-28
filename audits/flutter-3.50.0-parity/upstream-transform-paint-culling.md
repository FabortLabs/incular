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

## Finite matrices with unrepresentable bounds

An affine can contain only finite numbers and still overflow when
mapped into Incular's f32 world rectangles. A scale of half
`f32::MAX` and a translation of `1e40` both reproduced the
geometry panic in a previously painted clip subtree.

The new checked projection keeps ordinary bounds arithmetic and
the strict geometry API unchanged, while retained culling,
compositor bounds, semantic clips and oval scissors can reject an
unrepresentable rectangle safely. An intermediate overflow
disables the shortcut rather than dropping the subtree: the
`1e39` and `1e-39` composition still paints at its representable
world bounds. Crossing widely separated sibling boxes also disables
union culling while retaining the visible sibling. Cached pictures
reappear when their transform returns to range.

All 40 focused compositor, transform, annotation, and wgpu
offscreen checks passed. These authored range cases do not add
upstream registrations.

## Finite singular matrices with explicit filter quality

The authored pinned `singular_sampled_transform_test.dart` separates
a finite singular unfiltered transform from the same matrix with low
filter quality. Flutter calls the painter zero times without sampling,
and once with sampling; the sampled case records an image-filter
layer. The nonzero 0.25× control also records one stage.

Incular had been rejecting the sampled source alongside the unsampled
singular case. It now records the untransformed source and an empty
filtered output. wgpu still rejects the non-invertible filter before
allocating an offscreen source. The native tests prove the three
modes under and without an ancestor clip, balanced filter commands,
correct 30×20 source and 7.5×20 nonzero output, and correct invalidation
when changing only the sampling option with an already cached clip.
All 13 focused widget checks passed. This is distinct from original
:471 (which had no filter quality) and adds no upstream count.
