import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('A second hover in the same scrollbar cancels a new fade timer', (
    tester,
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
            child: SingleChildScrollView(
              controller: controller,
              child: const SizedBox(height: 4000, width: 4000),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    final scrollbar = find.byType(RawScrollbar);
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer();
    addTearDown(mouse.removePointer);
    await mouse.moveTo(const Offset(795, 5));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    void expectThumb(double top, Color color) {
      expect(
        scrollbar,
        paints
          ..rect(rect: const Rect.fromLTRB(794, 0, 800, 600))
          ..rect(rect: Rect.fromLTWH(794, top, 6, 90), color: color),
      );
    }

    const full = Color(0x66BCBCBC);
    expectThumb(0, full);
    controller.jumpTo(20);
    await tester.pump();
    expectThumb(3, full);
    await tester.pump(const Duration(milliseconds: 290));
    await mouse.moveTo(const Offset(796, 6));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1310));
    expectThumb(3, full);

    await mouse.moveTo(const Offset(750, 5));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 599));
    expectThumb(3, full);
    await tester.pump(const Duration(milliseconds: 1));
    await tester.pump(const Duration(milliseconds: 150));
    expectThumb(3, const Color(0x4FBCBCBC));
  });
}
