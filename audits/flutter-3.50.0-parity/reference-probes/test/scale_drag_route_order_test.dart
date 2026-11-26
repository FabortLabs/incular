import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('an earlier drag can win a new pointer before scale restarts', (
    WidgetTester tester,
  ) async {
    final trace = <String>[];
    await tester.pumpWidget(
      GestureDetector(
        behavior: HitTestBehavior.opaque,
        onScaleStart: (details) => trace.add('start ${details.pointerCount}'),
        onScaleUpdate: (details) => trace.add('update ${details.pointerCount}'),
        onScaleEnd: (details) => trace.add('end ${details.pointerCount}'),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onHorizontalDragStart: (_) => trace.add('drag'),
          child: const SizedBox.expand(),
        ),
      ),
    );
    final first = await tester.startGesture(const Offset(10, 10));
    expect(trace, isEmpty);
    await first.moveTo(const Offset(10, 50));
    expect(trace, ['start 1', 'update 1']);
    trace.clear();

    final second = await tester.startGesture(const Offset(10, 20));
    expect(trace, ['end 2']);
    trace.clear();
    await second.moveTo(const Offset(30, 20));
    expect(trace, ['drag']);
    trace.clear();

    await first.moveTo(const Offset(10, 90));
    expect(trace, ['start 1', 'update 1']);
    trace.clear();
    await first.up();
    expect(trace, ['end 0']);
    await second.up();
    expect(trace, ['end 0']);
  });
}
