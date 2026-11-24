import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Future<void> mountWheel(
  WidgetTester tester,
  FixedExtentScrollController controller,
  List<int> selected,
) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = const Size(800, 600);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(controller.dispose);
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: ListWheelScrollView(
        controller: controller,
        physics: const ClampingScrollPhysics(),
        itemExtent: 50,
        onSelectedItemChanged: selected.add,
        children: List<Widget>.generate(
          40,
          (index) => SizedBox(
            width: 300,
            child: ColoredBox(
              color: index.isEven
                  ? const Color(0xFF2196F3)
                  : const Color(0xFF4CAF50),
            ),
          ),
        ),
      ),
    ),
  );
  expect(controller.hasClients, isTrue);
  expect(controller.position.viewportDimension, 600);
  expect(controller.position.maxScrollExtent, 1950);
  expect(controller.offset, 0);
  expect(selected, isEmpty);
}

void main() {
  testWidgets('touch drag drives attached ListWheel before release', (
    tester,
  ) async {
    final controller = FixedExtentScrollController();
    final selected = <int>[];
    await mountWheel(tester, controller, selected);
    const step = Duration(milliseconds: 16);
    await tester.pump(step);
    final gesture = await tester.createGesture(kind: PointerDeviceKind.touch);
    await gesture.down(const Offset(400, 450), timeStamp: step);
    await tester.pump(step);
    await gesture.moveTo(const Offset(400, 450 - 200 / 3), timeStamp: step * 2);
    await tester.pump(step);

    expect(controller.offset, closeTo(200 / 3, 0.001));
    expect(controller.selectedItem, 1);
    expect(selected, [1]);
    await gesture.cancel();
  });

  testWidgets('programmatic jump drives attached ListWheel', (tester) async {
    final controller = FixedExtentScrollController();
    final selected = <int>[];
    await mountWheel(tester, controller, selected);
    controller.jumpTo(100);
    await tester.pump();
    expect(controller.offset, 100);
    expect(controller.selectedItem, 2);
    expect(selected, [2]);
  });
}
