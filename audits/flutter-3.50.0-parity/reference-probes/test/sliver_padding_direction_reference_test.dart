import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

class _Probe extends RenderSliver {
  @override
  void performLayout() {
    geometry = const SliverGeometry(
      paintOrigin: 10,
      paintExtent: 10,
      maxPaintExtent: 10,
    );
  }
}

void main() {
  testWidgets('asymmetric sliver padding consumes the actual leading edge', (
    tester,
  ) async {
    for (final (axis, cross, before) in [
      (AxisDirection.down, AxisDirection.right, 20.0),
      (AxisDirection.up, AxisDirection.right, 40.0),
      (AxisDirection.right, AxisDirection.down, 11.0),
      (AxisDirection.left, AxisDirection.down, 31.0),
    ]) {
      final probe = _Probe();
      final padding = RenderSliverPadding(
        padding: const EdgeInsets.fromLTRB(11, 20, 31, 40),
        child: probe,
      );
      addTearDown(probe.dispose);
      addTearDown(padding.dispose);
      padding.layout(
        SliverConstraints(
          axisDirection: axis,
          growthDirection: GrowthDirection.forward,
          userScrollDirection: ScrollDirection.idle,
          scrollOffset: 0,
          precedingScrollExtent: 0,
          overlap: 100,
          remainingPaintExtent: 100,
          crossAxisExtent: 100,
          crossAxisDirection: cross,
          viewportMainAxisExtent: 100,
          remainingCacheExtent: 100,
          cacheOrigin: 0,
        ),
        parentUsesSize: true,
      );
      expect(probe.constraints.precedingScrollExtent, before);
      expect(probe.constraints.overlap, 100 - before);
      expect(probe.constraints.remainingPaintExtent, 100 - before);
      expect(padding.geometry!.paintOrigin, 10);
    }
  });
}
