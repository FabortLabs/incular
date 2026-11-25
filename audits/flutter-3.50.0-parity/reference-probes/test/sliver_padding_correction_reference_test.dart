import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'padding forwards the retained correction after a leading box grows',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);

      Widget scene(double firstHeight) => Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          controller: controller,
          cacheExtent: 0,
          slivers: [
            SliverPadding(
              padding: EdgeInsets.zero,
              sliver: SliverList(
                delegate: SliverChildBuilderDelegate(
                  (_, index) => SizedBox(
                    key: ValueKey(index),
                    height: index == 0 ? firstHeight : 200,
                    child: Center(child: Text('$index')),
                  ),
                  childCount: 10,
                ),
              ),
            ),
          ],
        ),
      );

      await tester.pumpWidget(scene(200));
      controller.jumpTo(300);
      await tester.pump();
      expect(
        tester.getRect(find.byKey(const ValueKey(2))),
        const Rect.fromLTWH(0, 100, 800, 200),
      );

      await tester.pumpWidget(scene(400));
      controller.jumpTo(0);
      await tester.pump();
      expect(
        tester.getRect(find.byKey(const ValueKey(0))),
        const Rect.fromLTWH(0, -200, 800, 400),
      );
    },
  );
}
