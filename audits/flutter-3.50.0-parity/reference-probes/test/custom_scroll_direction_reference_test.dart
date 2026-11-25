import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  Future<void> compare(
    WidgetTester tester, {
    required bool reverse,
    required bool raw,
    required bool padding,
    required List<double> starts,
  }) async {
    const firstKey = ValueKey<String>('first');
    final controller = ScrollController();
    final rawOffset = ViewportOffset.zero();
    addTearDown(controller.dispose);
    addTearDown(rawOffset.dispose);
    int firstHits = 0;
    Widget first = SliverToBoxAdapter(
      child: Listener(
        key: firstKey,
        behavior: HitTestBehavior.opaque,
        onPointerDown: (_) => firstHits += 1,
        child: const SizedBox(width: 250, height: 100),
      ),
    );
    if (padding) {
      first = SliverPadding(
        padding: const EdgeInsetsDirectional.only(start: 20, end: 5),
        sliver: first,
      );
    }
    final slivers = <Widget>[
      first,
      const SliverToBoxAdapter(child: SizedBox(width: 250, height: 100)),
    ];
    final view = raw
        ? Viewport(
            axisDirection: reverse ? AxisDirection.left : AxisDirection.right,
            offset: rawOffset,
            slivers: slivers,
          )
        : CustomScrollView(
            scrollDirection: Axis.horizontal,
            reverse: reverse,
            controller: controller,
            slivers: slivers,
          );
    Widget scene(TextDirection direction) => Directionality(
      textDirection: direction,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(width: 300, height: 100, child: view),
      ),
    );
    const directions = [
      TextDirection.ltr,
      TextDirection.rtl,
      TextDirection.ltr,
    ];
    final finder = find.byKey(firstKey);
    await tester.pumpWidget(scene(directions.first));
    final firstRender = tester.renderObject<RenderBox>(finder);
    for (int i = 0; i < directions.length; i++) {
      await tester.pumpWidget(scene(directions[i]));
      expect(tester.renderObject<RenderBox>(finder), same(firstRender));
      final double start = starts[i];
      expect(tester.getRect(finder), Rect.fromLTWH(start, 0, 250, 100));
      expect(raw ? rawOffset.pixels : controller.offset, 0);
      for (final (double x, bool expected) in [
        (start + 1, true),
        (start + 249, true),
        (1.0, start == 0),
        (299.0, start == 50),
      ]) {
        firstHits = 0;
        await tester.tapAt(Offset(x, 50));
        expect(firstHits, expected ? 1 : 0, reason: 'x=$x start=$start');
      }
      if (!raw) {
        controller.jumpTo(30);
        await tester.pump();
        final bool leftward = reverse != (directions[i] == TextDirection.rtl);
        expect(
          tester.getRect(finder),
          Rect.fromLTWH(start + (leftward ? 30 : -30), 0, 250, 100),
        );
        controller.jumpTo(0);
        await tester.pump();
      }
    }
  }

  testWidgets('CustomScrollView horizontal ambient direction and reverse', (
    WidgetTester tester,
  ) async {
    await compare(
      tester,
      reverse: false,
      raw: false,
      padding: false,
      starts: [0, 50, 0],
    );
    await compare(
      tester,
      reverse: true,
      raw: false,
      padding: false,
      starts: [50, 0, 50],
    );
  });

  testWidgets('logical edges resolve independently of physical viewport axis', (
    WidgetTester tester,
  ) async {
    await compare(
      tester,
      reverse: false,
      raw: false,
      padding: true,
      starts: [20, 30, 20],
    );
    await compare(
      tester,
      reverse: true,
      raw: false,
      padding: true,
      starts: [45, 5, 45],
    );
    await compare(
      tester,
      reverse: false,
      raw: true,
      padding: true,
      starts: [20, 5, 20],
    );
    await compare(
      tester,
      reverse: true,
      raw: true,
      padding: true,
      starts: [45, 30, 45],
    );
  });
}
