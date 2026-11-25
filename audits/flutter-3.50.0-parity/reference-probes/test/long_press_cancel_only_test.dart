import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('long press cancellation does not depend on a down callback', (
    tester,
  ) async {
    const target = ValueKey<String>('target');
    var cancels = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: GestureDetector(
            key: target,
            behavior: HitTestBehavior.opaque,
            onLongPressCancel: () => cancels++,
            child: const SizedBox(width: 200, height: 200),
          ),
        ),
      ),
    );
    final center = tester.getCenter(find.byKey(target));
    Future<TestGesture> start() async {
      final touch = await tester.startGesture(center);
      await tester.pump(const Duration(milliseconds: 100));
      return touch;
    }

    final lifted = await start();
    await lifted.up();
    expect(cancels, 1);

    final moved = await start();
    await moved.moveBy(const Offset(100, 100));
    expect(cancels, 2);
    await moved.up();
    expect(cancels, 2);

    final cancelled = await start();
    await cancelled.cancel();
    expect(cancels, 3);

    final accepted = await start();
    await tester.pump(const Duration(milliseconds: 450));
    await accepted.up();
    expect(cancels, 3);
  });
}
