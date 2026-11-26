import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Header extends SliverPersistentHeaderDelegate {
  const _Header(this.minExtent, this.maxExtent, this.child);

  @override
  final double minExtent;
  @override
  final double maxExtent;
  final Widget child;

  @override
  Widget build(
    BuildContext context,
    double shrinkOffset,
    bool overlapsContent,
  ) => child;

  @override
  bool shouldRebuild(_Header oldDelegate) =>
      oldDelegate.minExtent != minExtent ||
      oldDelegate.maxExtent != maxExtent ||
      oldDelegate.child != child;
}

Widget _scene(
  ScrollController controller,
  List<Widget> lanes, {
  bool reverse = false,
}) {
  return Directionality(
    textDirection: TextDirection.ltr,
    child: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: 300,
        height: 600,
        child: CustomScrollView(
          controller: controller,
          reverse: reverse,
          slivers: <Widget>[
            SliverCrossAxisGroup(slivers: lanes),
            const SliverToBoxAdapter(child: SizedBox(height: 2400)),
          ],
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('cross-axis pin uses the physical paint offset when reversed', (
    WidgetTester tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    const key = ValueKey<String>('pin');
    await tester.pumpWidget(
      _scene(controller, <Widget>[
        const SliverToBoxAdapter(child: SizedBox(height: 600)),
        SliverPersistentHeader(
          pinned: true,
          delegate: const _Header(40, 60, SizedBox(key: key, height: 60)),
        ),
      ], reverse: true),
    );

    for (final (double scroll, double expectedY) in <(double, double)>[
      (570, 560),
      (550, 550),
    ]) {
      controller.jumpTo(scroll);
      await tester.pumpAndSettle();
      expect(
        tester.getRect(find.byKey(key)),
        Rect.fromLTWH(150, expectedY, 150, 40),
      );
      final header = tester.renderObject<RenderSliver>(
        find.byType(SliverPersistentHeader),
      );
      expect(header.geometry!.paintExtent, 40);
    }
  });

  testWidgets('pinned headers in neighboring lanes share a leading edge', (
    WidgetTester tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    const left = ValueKey<String>('left');
    const right = ValueKey<String>('right');
    await tester.pumpWidget(
      _scene(controller, <Widget>[
        const SliverToBoxAdapter(child: SizedBox(height: 600)),
        SliverPersistentHeader(
          pinned: true,
          delegate: const _Header(60, 60, SizedBox(key: left, height: 60)),
        ),
        SliverPersistentHeader(
          pinned: true,
          delegate: const _Header(40, 40, SizedBox(key: right, height: 40)),
        ),
      ]),
    );

    controller.jumpTo(200);
    await tester.pumpAndSettle();
    expect(
      tester.getRect(find.byKey(left)),
      const Rect.fromLTWH(100, 0, 100, 60),
    );
    expect(
      tester.getRect(find.byKey(right)),
      const Rect.fromLTWH(200, 0, 100, 40),
    );

    controller.jumpTo(580);
    await tester.pumpAndSettle();
    expect(
      tester.getRect(find.byKey(left)),
      const Rect.fromLTWH(100, -40, 100, 60),
    );
    expect(
      tester.getRect(find.byKey(right)),
      const Rect.fromLTWH(200, -20, 100, 40),
    );
  });
}
