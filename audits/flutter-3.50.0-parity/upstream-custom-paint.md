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
