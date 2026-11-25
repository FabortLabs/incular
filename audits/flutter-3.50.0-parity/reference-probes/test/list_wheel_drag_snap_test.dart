import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a wheel drag snaps to the nearest item without release velocity',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetDevicePixelRatio);
      final controller = FixedExtentScrollController();
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: ListWheelScrollView(
            controller: controller,
            physics: const FixedExtentScrollPhysics(),
            itemExtent: 100,
            children: List.generate(30, (_) => const SizedBox.expand()),
          ),
        ),
      );
      final gesture = await tester.createGesture();
      await gesture.down(
        const Offset(10, 150),
        timeStamp: const Duration(milliseconds: 16),
      );
      await gesture.moveTo(
        const Offset(10, 90),
        timeStamp: const Duration(milliseconds: 32),
      );
      await tester.pump();
      expect(controller.offset, 60);
      expect(controller.selectedItem, 1);
      await gesture.up(timeStamp: const Duration(milliseconds: 250));
      await tester.pumpAndSettle();
      expect(controller.offset, closeTo(100, 1));
      expect(controller.selectedItem, 1);
    },
  );
}
