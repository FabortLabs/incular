import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('A zero-width PageView caches the fractional logical page', (
    WidgetTester tester,
  ) async {
    final controller = PageController(viewportFraction: 1.25);
    addTearDown(controller.dispose);
    final log = <int>[];
    Widget build(double width) => Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: SizedBox(
          width: width,
          height: 200,
          child: PageView(
            controller: controller,
            pageSnapping: false,
            onPageChanged: log.add,
            children: List<Widget>.generate(
              8,
              (int page) => Text('Page $page'),
            ),
          ),
        ),
      ),
    );

    await tester.pumpWidget(build(200));
    controller.jumpTo(425);
    await tester.pump();
    expect(controller.page, closeTo(1.6, 1e-12));
    expect(log, <int>[2]);

    await tester.pumpWidget(build(0));
    expect(controller.page, closeTo(1.7, 1e-12));
    await tester.pumpWidget(build(300));
    expect(controller.page, closeTo(1.7, 1e-12));
    expect(controller.position.pixels, 675);
    expect(log, <int>[2]);
  });
}
