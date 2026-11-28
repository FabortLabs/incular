import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Held raw track press pages once before release', (
    WidgetTester tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: MediaQuery(
          data: const MediaQueryData(),
          child: RawScrollbar(
            controller: controller,
            thumbVisibility: true,
            child: SingleChildScrollView(
              controller: controller,
              child: const SizedBox(width: 4000, height: 4000),
            ),
          ),
        ),
      ),
    );

    await tester.pumpAndSettle();
    final pointer = await tester.startGesture(const Offset(797, 500));
    await tester.pump(const Duration(milliseconds: 99));
    expect(controller.offset, 0);
    await tester.pump(const Duration(milliseconds: 1));
    await tester.pump(const Duration(milliseconds: 50));
    expect(controller.offset, inInclusiveRange(230, 255));
    await tester.pump(const Duration(milliseconds: 60));
    expect(controller.offset, 480);
    await pointer.up();
    await tester.pumpAndSettle();
    expect(controller.offset, 480);
  });
}
