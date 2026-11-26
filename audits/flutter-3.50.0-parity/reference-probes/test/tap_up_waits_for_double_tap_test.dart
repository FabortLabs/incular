import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('tap up waits for a double-tap competitor to leave the arena', (
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
            onTap: () => log.add('single'),
            onTapCancel: () => log.add('cancel'),
            onDoubleTap: () => log.add('double'),
            child: const SizedBox(width: 120, height: 120),
          ),
        ),
      ),
    );

    Future<void> press() async {
      final gesture = await tester.startGesture(const Offset(400, 300));
      await tester.pump(const Duration(milliseconds: 10));
      await gesture.up();
      await tester.pump();
    }

    await press();
    expect(log, isEmpty);
    await tester.pump(const Duration(milliseconds: 100));
    expect(log, isEmpty);
    await tester.pump(const Duration(milliseconds: 200));
    expect(log, ['down', 'up', 'single']);

    log.clear();
    await press();
    await tester.pump(const Duration(milliseconds: 40));
    await press();
    expect(log, ['double']);
    await tester.pump(const Duration(milliseconds: 350));
    expect(log, ['double']);

    log.clear();
    final held = await tester.startGesture(const Offset(400, 300));
    await tester.pump(const Duration(milliseconds: 110));
    expect(log, ['down']);
    await held.up();
    expect(log, ['down']);
    await tester.pump(const Duration(milliseconds: 300));
    expect(log, ['down', 'up', 'single']);

    log.clear();
    final first = await tester.startGesture(const Offset(400, 300));
    await tester.pump(const Duration(milliseconds: 110));
    await first.up();
    expect(log, ['down']);
    await tester.pump(const Duration(milliseconds: 40));
    await press();
    expect(log, ['down', 'double']);
    await tester.pump(const Duration(milliseconds: 50));
    expect(log, ['down', 'double']);
  });
}
