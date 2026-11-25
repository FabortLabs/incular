import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('each new wheel position learns the actual item extent', (
    tester,
  ) async {
    final controller = FixedExtentScrollController(initialItem: 3);
    addTearDown(controller.dispose);
    Widget wheel(double extent) => Directionality(
      textDirection: TextDirection.ltr,
      child: ListWheelScrollView(
        controller: controller,
        itemExtent: extent,
        children: List.generate(100, (_) => const SizedBox.expand()),
      ),
    );
    await tester.pumpWidget(wheel(80));
    final first = controller.position;
    expect(controller.selectedItem, 3);
    expect(controller.offset, 240);
    controller.jumpToItem(10);
    await tester.pump();
    expect(controller.selectedItem, 10);
    expect(controller.offset, 800);
    expect(controller.position, same(first));

    await tester.pumpWidget(const SizedBox());
    expect(controller.hasClients, isFalse);
    await tester.pumpWidget(wheel(120));
    expect(controller.selectedItem, 3);
    expect(controller.offset, 360);
    expect(controller.position, isNot(same(first)));
  });
}
