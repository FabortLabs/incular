import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('PageView notifications expose their own page metrics', (
    tester,
  ) async {
    final controller = PageController(viewportFraction: 1.25);
    addTearDown(controller.dispose);
    final captured = <ScrollMetrics>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: SizedBox(
            width: 200,
            height: 100,
            child: NotificationListener<ScrollUpdateNotification>(
              onNotification: (event) {
                captured.add(event.metrics);
                return false;
              },
              child: PageView(
                pageSnapping: false,
                controller: controller,
                children: const [Text('0'), Text('1'), Text('2'), Text('3')],
              ),
            ),
          ),
        ),
      ),
    );
    captured.clear();
    controller.jumpTo(425);
    final snapshot = captured.single as PageMetrics;
    expect(snapshot.viewportFraction, 1.25);
    expect(snapshot.axisDirection, AxisDirection.right);
    expect(snapshot.pixels, 425);
    expect(snapshot.page, 1.7);
    expect(controller.page, 1.6);
    final moved = snapshot.copyWith(pixels: 500);
    expect(moved.page, 2);
    expect(snapshot.page, 1.7);
  });
}
