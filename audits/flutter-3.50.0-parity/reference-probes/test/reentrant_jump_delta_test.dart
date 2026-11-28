import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a jump update measures from the prior position after reentry', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final positions = <double>[];
    final updates = <(double?, double)>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: NotificationListener<ScrollUpdateNotification>(
          onNotification: (event) {
            updates.add((event.scrollDelta, event.metrics.pixels));
            return false;
          },
          child: ListView(
            controller: controller,
            children: const [SizedBox(height: 2000)],
          ),
        ),
      ),
    );

    controller.addListener(() {
      final position = controller.offset;
      positions.add(position);
      if (position == 100) {
        controller.jumpTo(200);
      }
    });
    controller.jumpTo(100);
    expect(controller.offset, 200);
    expect(positions, [100, 200]);
    expect(updates, [(100, 200), (200, 200)]);
  });
}
