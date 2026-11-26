import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final (behavior, opaque, expectedHover, expectedTap) in [
    (HitTestBehavior.deferToChild, true, ['behind'], 1),
    (HitTestBehavior.opaque, true, ['front'], 0),
    (HitTestBehavior.opaque, false, ['behind', 'front'], 1),
    (HitTestBehavior.translucent, true, ['behind', 'front'], 1),
  ]) {
    testWidgets('overlap with ignored child: $behavior, opaque=$opaque', (
      tester,
    ) async {
      final hovered = <String>[];
      var taps = 0;
      await tester.pumpWidget(
        Directionality(
          textDirection: ui.TextDirection.ltr,
          child: Stack(
            children: [
              Positioned(
                left: 0,
                top: 0,
                child: MouseRegion(
                  onEnter: (_) => hovered.add('behind'),
                  child: GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onTap: () => taps++,
                    child: const SizedBox(width: 60, height: 60),
                  ),
                ),
              ),
              Positioned(
                left: 20,
                top: 20,
                child: MouseRegion(
                  opaque: opaque,
                  hitTestBehavior: behavior,
                  onEnter: (_) => hovered.add('front'),
                  child: GestureDetector(
                    behavior: HitTestBehavior.deferToChild,
                    onTap: () {},
                    child: const IgnorePointer(
                      child: SizedBox(width: 50, height: 50),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      const point = ui.Offset(25, 25);
      final pointer = await tester.createGesture(
        kind: ui.PointerDeviceKind.mouse,
      );
      await pointer.addPointer(location: const ui.Offset(80, 80));
      await pointer.moveTo(point);
      await tester.pump();
      expect(hovered, expectedHover);
      await pointer.removePointer();
      await tester.tapAt(point);
      expect(taps, expectedTap);
    });
  }

  testWidgets('an absorber blocks only within its half-open layout box', (
    tester,
  ) async {
    var taps = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: ui.TextDirection.ltr,
        child: Stack(
          children: [
            GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => taps++,
              child: const SizedBox(width: 100, height: 100),
            ),
            const Positioned(
              left: 10,
              top: 50,
              child: AbsorbPointer(child: SizedBox(width: 40, height: 30)),
            ),
          ],
        ),
      ),
    );
    for (final (x, y, expected) in <(double, double, int)>[
      (20, 20, 1),
      (10, 50, 1),
      (49.99, 79.99, 1),
      (50, 60, 2),
      (20, 80, 3),
      (9.99, 60, 4),
    ]) {
      await tester.tapAt(ui.Offset(x, y));
      expect(taps, expected, reason: 'at ($x, $y)');
    }
  });
}
