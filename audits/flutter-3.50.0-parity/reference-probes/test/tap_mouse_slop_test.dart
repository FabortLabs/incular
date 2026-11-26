import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('ordinary mouse taps use the eighteen pixel tap tolerance', (
    tester,
  ) async {
    final log = <String>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapDown: (_) => log.add('down'),
            onTapUp: (_) => log.add('up'),
            onTap: () => log.add('tap'),
            onTapCancel: () => log.add('cancel'),
            child: const SizedBox(width: 100, height: 100),
          ),
        ),
      ),
    );
    for (final (double distance, List<String> expected) in [
      (2.0, ['down', 'up', 'tap']),
      (18.0, ['down', 'up', 'tap']),
      (18.25, ['down', 'cancel']),
    ]) {
      log.clear();
      final mouse = await tester.startGesture(
        const Offset(400, 300),
        kind: PointerDeviceKind.mouse,
      );
      await tester.pump();
      await mouse.moveBy(Offset(distance, 0));
      await mouse.up();
      await tester.pump();
      expect(log, expected, reason: 'distance $distance');
      await mouse.removePointer();
    }
  });
}
