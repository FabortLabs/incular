import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _OnePixelOverlap extends LeafRenderObjectWidget {
  const _OnePixelOverlap();

  @override
  RenderSliver createRenderObject(BuildContext context) => _RenderOverlap();
}

class _RenderOverlap extends RenderSliver {
  @override
  void performLayout() {
    geometry = const SliverGeometry(
      paintExtent: 1,
      layoutExtent: 0,
      maxPaintExtent: 1,
    );
  }
}

void main() {
  testWidgets('shrink-wrapping clips overlap only under a finite main limit', (
    tester,
  ) async {
    const full = Rect.fromLTWH(0, 0, 800, 600);
    final viewportKey = GlobalKey();
    final childKey = GlobalKey();
    for (final direction in AxisDirection.values) {
      final vertical = axisDirectionToAxis(direction) == Axis.vertical;
      for (final unbounded in [false, true, false]) {
        await tester.pumpWidget(
          OverflowBox(
            alignment: Alignment.topLeft,
            minWidth: 800,
            maxWidth: unbounded && !vertical ? double.infinity : 800,
            minHeight: 600,
            maxHeight: unbounded && vertical ? double.infinity : 600,
            child: ShrinkWrappingViewport(
              key: viewportKey,
              axisDirection: direction,
              crossAxisDirection: vertical
                  ? AxisDirection.left
                  : AxisDirection.up,
              offset: ViewportOffset.zero(),
              slivers: [
                const _OnePixelOverlap(),
                SliverToBoxAdapter(
                  key: childKey,
                  child: const SizedBox.square(dimension: 40),
                ),
              ],
            ),
          ),
        );
        final viewport = tester.renderObject<RenderShrinkWrappingViewport>(
          find.byKey(viewportKey),
        );
        final child = tester.renderObject<RenderSliver>(find.byKey(childKey));
        expect(viewport.size, full.size);
        expect(child.constraints.overlap, 1);
        expect(child.constraints.viewportMainAxisExtent.isInfinite, unbounded);
        final bounded = switch (direction) {
          AxisDirection.up => const Rect.fromLTRB(0, 0, 800, 599),
          AxisDirection.right => const Rect.fromLTRB(1, 0, 800, 600),
          AxisDirection.down => const Rect.fromLTRB(0, 1, 800, 600),
          AxisDirection.left => const Rect.fromLTRB(0, 0, 799, 600),
        };
        expect(
          viewport.describeApproximatePaintClip(child),
          unbounded ? full : bounded,
        );
      }
    }
  });
}
