import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final horizontal in <bool>[true, false]) {
    testWidgets(
      '${horizontal ? 'horizontal' : 'pan'} tap-drag preserves both delta axes',
      (tester) async {
        final updates = <TapDragUpdateDetails>[];
        void configure(BaseTapAndDragGestureRecognizer recognizer) {
          recognizer
            ..dragStartBehavior = DragStartBehavior.down
            ..onDragUpdate = updates.add;
        }

        await tester.pumpWidget(
          Transform.scale(
            scale: 2,
            alignment: Alignment.topLeft,
            child: RawGestureDetector(
              behavior: HitTestBehavior.opaque,
              gestures: <Type, GestureRecognizerFactory>{
                if (horizontal)
                  TapAndHorizontalDragGestureRecognizer:
                      GestureRecognizerFactoryWithHandlers<
                        TapAndHorizontalDragGestureRecognizer
                      >(TapAndHorizontalDragGestureRecognizer.new, configure)
                else
                  TapAndPanGestureRecognizer:
                      GestureRecognizerFactoryWithHandlers<
                        TapAndPanGestureRecognizer
                      >(TapAndPanGestureRecognizer.new, configure),
              },
            ),
          ),
        );
        await tester.sendEventToBinding(
          const PointerDownEvent(pointer: 1, position: Offset(32, 28)),
        );
        await tester.sendEventToBinding(
          const PointerMoveEvent(
            pointer: 1,
            position: Offset(72, 48),
            delta: Offset(40, 20),
            timeStamp: Duration(milliseconds: 11),
          ),
        );
        await tester.sendEventToBinding(
          const PointerMoveEvent(
            pointer: 1,
            position: Offset(80, 42),
            delta: Offset(8, -6),
            timeStamp: Duration(milliseconds: 27),
          ),
        );
        await tester.sendEventToBinding(
          const PointerUpEvent(pointer: 1, position: Offset(80, 42)),
        );
        expect(updates, hasLength(2));
        for (final (i, ms, position, delta, travel) in <
          (int, int, Offset, Offset, Offset)
        >[
          (0, 11, Offset(72, 48), Offset(20, 10), Offset(40, 20)),
          (1, 27, Offset(80, 42), Offset(4, -3), Offset(48, 14)),
        ]) {
          final details = updates[i];
          expect(details.sourceTimeStamp, Duration(milliseconds: ms));
          expect(details.globalPosition, position);
          expect(details.localPosition, position / 2);
          expect(details.delta, delta);
          expect(details.primaryDelta, isNull);
          expect(details.offsetFromOrigin, travel);
          expect(details.localOffsetFromOrigin, travel / 2);
          expect(details.kind, PointerDeviceKind.touch);
          expect(details.consecutiveTapCount, 1);
        }
      },
    );
  }
}
