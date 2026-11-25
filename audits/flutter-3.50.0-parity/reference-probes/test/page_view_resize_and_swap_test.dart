import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget host(PageController controller, double width, {bool snapping = true}) =>
    Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: SizedBox(
          width: width,
          height: 100,
          child: PageView(
            controller: controller,
            pageSnapping: snapping,
            children: List.generate(6, (i) => Text('page $i')),
          ),
        ),
      ),
    );

void main() {
  testWidgets(
    'controller replacement while collapsed keeps the selected page',
    (tester) async {
      final first = PageController(initialPage: 2);
      final second = PageController(initialPage: 4, viewportFraction: 0.75);
      addTearDown(first.dispose);
      addTearDown(second.dispose);

      await tester.pumpWidget(host(first, 200));
      expect(first.page, 2);
      await tester.pumpWidget(host(first, 0));
      expect(first.page, 2);
      await tester.pumpWidget(host(second, 0));
      expect(first.hasClients, isFalse);
      expect(second.page, 2);

      await tester.pumpWidget(host(second, 300));
      expect(second.page, 2);
      expect(second.offset, 450);
    },
  );

  for (final snapping in [true, false]) {
    testWidgets('resize reapplies fractional position (snapping=$snapping)', (
      tester,
    ) async {
      final controller = PageController(viewportFraction: 1.25);
      addTearDown(controller.dispose);
      await tester.pumpWidget(host(controller, 200, snapping: snapping));
      // jumpTo triggers a ballistic when snapping is enabled. Resize before
      // pumping a settling frame; the position itself must use the new size.
      controller.jumpTo(425);
      expect(controller.page, 1.6);
      await tester.pumpWidget(host(controller, 300, snapping: snapping));
      expect(controller.page, closeTo(1.55, 0.000000001));
      expect(controller.offset, closeTo(618.75, 0.000000001));
    });
  }
}
