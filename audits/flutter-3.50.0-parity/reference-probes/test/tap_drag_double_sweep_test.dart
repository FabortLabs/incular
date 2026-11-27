import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a raw combined tap waits for the competing double tap sweep', (
    tester,
  ) async {
    final events = <String>[];
    await tester.pumpWidget(
      RawGestureDetector(
        behavior: HitTestBehavior.opaque,
        gestures: <Type, GestureRecognizerFactory>{
          TapAndPanGestureRecognizer:
              GestureRecognizerFactoryWithHandlers<TapAndPanGestureRecognizer>(
                TapAndPanGestureRecognizer.new,
                (recognizer) {
                  recognizer
                    ..dragStartBehavior = DragStartBehavior.down
                    ..eagerVictoryOnDrag = true
                    ..maxConsecutiveTap = 3
                    ..onTapDown = (details) {
                      events.add('down#${details.consecutiveTapCount}');
                    }
                    ..onTapUp = (details) {
                      events.add('up#${details.consecutiveTapCount}');
                    }
                    ..onDragStart = (details) {
                      events.add('panstart');
                    }
                    ..onDragUpdate = (details) {
                      events.add('panupdate');
                    }
                    ..onDragEnd = (details) {
                      events.add('panend');
                    }
                    ..onCancel = () {
                      events.add('cancel');
                    };
                },
              ),
          TapGestureRecognizer:
              GestureRecognizerFactoryWithHandlers<TapGestureRecognizer>(
                TapGestureRecognizer.new,
                (recognizer) {
                  recognizer
                    ..onTapDown = (_) {
                      events.add('tapdown');
                    }
                    ..onTapUp = (_) {
                      events.add('tapup');
                    }
                    ..onTapCancel = () {
                      events.add('tapscancel');
                    };
                },
              ),
          DoubleTapGestureRecognizer:
              GestureRecognizerFactoryWithHandlers<DoubleTapGestureRecognizer>(
                DoubleTapGestureRecognizer.new,
                (recognizer) {
                  recognizer
                    ..onDoubleTapDown = (_) {
                      events.add('doubletapdown');
                    }
                    ..onDoubleTap = () {
                      events.add('doubletapup');
                    }
                    ..onDoubleTapCancel = () {
                      events.add('doubletapcancel');
                    };
                },
              ),
        },
        child: const SizedBox.expand(),
      ),
    );

    await tester.sendEventToBinding(
      const PointerDownEvent(pointer: 1, position: Offset(10, 10)),
    );
    await tester.sendEventToBinding(
      const PointerUpEvent(pointer: 1, position: Offset(11, 9)),
    );
    expect(events, isEmpty);
    await tester.pump(kDoubleTapTimeout - const Duration(milliseconds: 1));
    expect(events, isEmpty);
    await tester.pump(const Duration(milliseconds: 1));
    expect(events, ['down#1', 'up#1']);
  });
}
