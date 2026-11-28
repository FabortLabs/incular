import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool capable in [true, false]) {
    testWidgets('first pressure sample competes with a tap: capable=$capable', (
      tester,
    ) async {
      final events = <String>[];
      await tester.pumpWidget(
        TextSelectionGestureDetector(
          behavior: HitTestBehavior.opaque,
          onDragSelectionStart: (_) {},
          onSingleTapUp: (_) => events.add('tap'),
          onForcePressStart: (_) => events.add('force start'),
          onForcePressEnd: (_) => events.add('force end'),
          child: const SizedBox.expand(),
        ),
      );
      final pointer = tester.nextPointer;
      final gesture = await tester.createGesture(pointer: pointer);
      const point = Offset(200, 200);
      await gesture.downWithCustomEvent(
        point,
        PointerDownEvent(
          pointer: pointer,
          position: point,
          pressure: capable ? 3 : 0.8,
          pressureMin: 0,
          pressureMax: capable ? 6 : 1,
        ),
      );
      await tester.pump();
      expect(events, capable ? ['force start'] : isEmpty);
      await gesture.up();
      await tester.pump();
      expect(events, capable ? ['force start', 'force end'] : ['tap']);
    });
  }
}
