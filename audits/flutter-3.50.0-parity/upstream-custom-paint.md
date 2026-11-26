# CustomPaint paint order and sizing

All eight originals in widgets/custom_paint_test.dart passed against pinned Flutter 3.50.0. The focused Rust runner passes all five tests.

- :43: real callbacks run in exactly background, child, foreground order.
- :124: six retained updates measure respectively 0×0, 800×600, 20×20, 800×100, 800×600, 0×0. The optional child overrides the preferred size and a preferred width of 2000 clamps to the available width.
- :198 and :206 are partial matches. The public IntrinsicWidth/IntrinsicHeight parents observe [20,30] for a finite preferred size and [0,0] for the fill-to-constraints analog to Size.infinite. The Rust fixtures don't invoke all four raw RenderCustomPaint min/max intrinsic methods separately.
- :214 matches the zero-area geometry and completes painting. This covers zero constraints, not a native window resize.
- :58, :166 and :193 remain reference-only. Incular painters return an independent display list, not Flutter's shared Canvas, and no equivalent public isComplex/willChange setter is present on this widget. Renderer performance or cache correspondence is not claimed.

Run Rust with cargo test --locked -p incular-widgets --test flutter350_custom_paint_parity -- --nocapture.

## Custom painter hit policy

Two authored pinned Flutter probes and two retained native tests use
actual touch taps. A background painter accepts hits by default even
with an empty display list, or can decline to let a sibling receive the
tap. A foreground painter defaults to leaving the child alone. Returning
true claims the point before the child; returning false leaves it to
the child. The delegate sees the local point. All 21 focused native
painter, semantics, and decorated-sliver tests passed.

## Independent paint and accessibility invalidation

The pinned authored probe and native regression hold semantic regions
while a replacement repaints, rebuild those regions on a size change
even when shouldRebuildSemantics returns false, and then change only
semantics while shouldRepaint returns false. The keyed region keeps its
node ID across all four states. The region-builder call trace matches
the three required sizes. The built regions are retained per mount
rather than shared by cloned descriptors.

The original `widgets/custom_painter_test.dart:795` is partial:
the behavior and region-builder gate match, but its extra assertion
on the number of times Flutter reads the semanticsBuilder getter is
not a Rust API contract. Incular retains the callback on each delegate.
All 13 focused native paint, hit, and semantics tests passed.

## Pinned custom painter semantics structure

Four original `widgets/custom_painter_test.dart` scenes pair:
:27 has no semantic node when neither painter supplies regions;
:37 and :75 expose a single 1×1 region at (1,1) within the full
800×600 paint boundary, including its RTL metadata. :113 groups the
background region, actual text child, then foreground region. The
retained paint's grouping preserves that order rather than letting
a geometric reading-order pass move both small regions ahead of the
text. All 35 focused native tests passed, including neighboring
sliver reading-order regressions that are intentionally unchanged.

## Region metadata, enablement, and retained identities

Paired `custom_painter_test.dart` :293, :336, :381, :725, and the
dynamic registration at :828 (all eight list changes). The painter
regions preserve the table role and the invalid editable-field
annotation. The same mounted painter supports semantics off, on, off,
and on. Duplicate region keys raise a structured generated-child error
that names the duplicate key. Keyed nodes survive insertion, removal,
and movement; unkeyed lists match the expected reading order.

:765 is partial: resizing rebuilds the output regions and matches the
laid-out sizes; Incular does not perform Flutter's repeated accesses
to its painter's semanticsBuilder getter. All 12 focused native
semantics tests passed against the selected pinned Flutter originals.

## Mounted painter repaint source

The authored Flutter case and native test change the actual painted
color from red to blue via a notification, without invoking the region
builder. A source replacement detaches the old source, leaves the
existing picture intact, and only the new source can repaint it green.
Native layout and rebuild counts stay unchanged; unmount releases the
last subscription. All 24 targeted native tests passed.

## Dispatching a painted region's accessibility actions

The pinned :425 registration pairs. The retained Runtime actually
dispatches all 23 actions advertised by a painted semantic region,
including the original selection base/extent, replacement text,
cursor-extension booleans, directional scrolling, and accessibility
focus notifications. The region requires no keyboard-focusable
editor. Its empty geometry boxes already pass ordinary touch to the
real child, so the accidental IgnorePointer action mask was removed.
The full 12-case native region target and this Runtime case passed.

## Painter property replacement

Paired :171. The same keyed 2×2 semantic region moves from (1,2)
to (5,6) while its label, value, both adjustment hints, descriptive
hint, text direction, checked and selected states, and eight actions
update. Explicit selected=false is no longer lost as an unspecified
flag. A desktop AccessKit control verifies absence → false → true →
absence on one unchanged native ID and confirms that the initial
false update emits a real patch. The web serialization follows the
existing role filter; browser execution was not verified in this
toolchain because its wasm standard library is not installed.

Thirteen focused painter tests and five desktop projection tests passed.
