import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

enum _ViewKind { list, grid, pages }

void main() {
  for (final _ViewKind kind in _ViewKind.values) {
    testWidgets('${kind.name} follows ambient horizontal direction', (
      WidgetTester tester,
    ) async {
      for (final bool reverse in [false, true]) {
        final ScrollController controller = kind == _ViewKind.pages
            ? PageController(viewportFraction: 0.8)
            : ScrollController();
        addTearDown(controller.dispose);
        const firstKey = ValueKey<String>('first');
        int firstHits = 0;
        final children = <Widget>[
          Listener(
            key: firstKey,
            behavior: HitTestBehavior.opaque,
            onPointerDown: (_) => firstHits++,
            child: const SizedBox(width: 250),
          ),
          const SizedBox(width: 250),
          const SizedBox(width: 250),
        ];
        final Widget view = switch (kind) {
          _ViewKind.list => ListView(
            scrollDirection: Axis.horizontal,
            reverse: reverse,
            itemExtent: 250,
            controller: controller,
            children: children,
          ),
          _ViewKind.grid => GridView(
            scrollDirection: Axis.horizontal,
            reverse: reverse,
            controller: controller,
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 1,
              mainAxisExtent: 250,
            ),
            children: children,
          ),
          _ViewKind.pages => PageView(
            reverse: reverse,
            padEnds: false,
            pageSnapping: false,
            controller: controller as PageController,
            children: children,
          ),
        };
        Widget scene(TextDirection direction) => Directionality(
          textDirection: direction,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(width: 300, height: 100, child: view),
          ),
        );
        final Finder first = find.byKey(firstKey);
        await tester.pumpWidget(scene(TextDirection.ltr));
        final render = tester.renderObject(first);
        final double width = kind == _ViewKind.pages ? 240 : 250;
        for (final direction in [
          TextDirection.ltr,
          TextDirection.rtl,
          TextDirection.ltr,
        ]) {
          await tester.pumpWidget(scene(direction));
          expect(tester.renderObject(first), same(render));
          final bool leftward = reverse != (direction == TextDirection.rtl);
          for (final double offset in [0, 30]) {
            controller.jumpTo(offset);
            await tester.pump();
            final double x = leftward ? 300 - width + offset : -offset;
            expect(tester.getRect(first), Rect.fromLTWH(x, 0, width, 100));
            firstHits = 0;
            await tester.tapAt(Offset(x + width / 2, 50));
            expect(firstHits, 1);
          }
        }
      }
    });
  }
}
