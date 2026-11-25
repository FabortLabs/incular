import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('scroll-end reporting waits for fixed-extent settlement', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetDevicePixelRatio);
    final selected = <int>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: ListWheelScrollView(
          itemExtent: 100,
          physics: const FixedExtentScrollPhysics(),
          changeReportingBehavior: ChangeReportingBehavior.onScrollEnd,
          onSelectedItemChanged: selected.add,
          children: List.generate(10, (_) => const SizedBox.expand()),
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
    await gesture.up(timeStamp: const Duration(milliseconds: 250));
    expect(selected, isEmpty);
    await tester.pumpAndSettle();
    expect(selected, [1]);
  });

  testWidgets('replacing a callback keeps the prior reported selection', (
    tester,
  ) async {
    final controller = FixedExtentScrollController();
    addTearDown(controller.dispose);
    final old = <int>[];
    final replacement = <int>[];
    Widget wheel(ValueChanged<int> selector) => Directionality(
      textDirection: TextDirection.ltr,
      child: ListWheelScrollView(
        itemExtent: 100,
        controller: controller,
        onSelectedItemChanged: selector,
        children: List.generate(10, (_) => const SizedBox.expand()),
      ),
    );
    await tester.pumpWidget(wheel(old.add));
    controller.jumpTo(70);
    expect(old, [1]);

    await tester.pumpWidget(wheel(replacement.add));
    controller.jumpTo(71);
    expect(replacement, isEmpty);
    controller.jumpTo(170);
    expect(replacement, [2]);
    expect(old, [1]);

    await tester.pumpWidget(const SizedBox());
    expect(controller.hasClients, isFalse);
  });
}
