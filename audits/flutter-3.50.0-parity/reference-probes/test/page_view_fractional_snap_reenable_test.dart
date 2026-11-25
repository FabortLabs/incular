import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Reenabling snapping aligns the current fractional page', (
    WidgetTester tester,
  ) async {
    final controller = PageController(viewportFraction: 0.75);
    addTearDown(controller.dispose);
    final log = <int>[];
    Widget build({required bool snapping}) => Directionality(
      textDirection: TextDirection.ltr,
      child: PageView(
        controller: controller,
        pageSnapping: snapping,
        onPageChanged: log.add,
        children: List<Widget>.generate(5, (int i) => Text('Page $i')),
      ),
    );

    await tester.pumpWidget(build(snapping: false));
    final TestGesture gesture = await tester.startGesture(
      const Offset(400, 300),
    );
    for (final double x in <double>[370, 130, -110, -350, -590]) {
      await gesture.moveTo(Offset(x, 300));
      await tester.pump(const Duration(milliseconds: 16));
    }
    await tester.pump(const Duration(milliseconds: 180));
    await gesture.up();
    await tester.pumpAndSettle();
    expect(controller.page, inExclusiveRange(1.5, 2.0));
    expect(log, <int>[1, 2]);

    await tester.pumpWidget(build(snapping: true));
    await tester.pumpAndSettle();
    expect(controller.page, 2);
    expect(controller.position.pixels, 1200);
    expect(log, <int>[1, 2]);
    expect(tester.getTopLeft(find.text('Page 2')).dx, 100);
  });
}
