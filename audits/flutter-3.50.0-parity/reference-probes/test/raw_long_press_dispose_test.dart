import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Unmounting a pending raw hold cancels it once without starting',
    (WidgetTester tester) async {
      final events = <String>[];
      await tester.pumpWidget(
        Center(
          child: RawGestureDetector(
            behavior: HitTestBehavior.opaque,
            gestures: <Type, GestureRecognizerFactory>{
              LongPressGestureRecognizer:
                  GestureRecognizerFactoryWithHandlers<
                    LongPressGestureRecognizer
                  >(LongPressGestureRecognizer.new, (recognizer) {
                    recognizer.onLongPressStart = (_) => events.add('start');
                    recognizer.onLongPressCancel = () => events.add('cancel');
                    recognizer.onLongPressEnd = (_) => events.add('end');
                  }),
            },
            child: const SizedBox(width: 100, height: 100),
          ),
        ),
      );
      final contact = await tester.startGesture(
        tester.getCenter(find.byType(RawGestureDetector)),
      );
      expect(events, isEmpty);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(events, ['cancel']);
      await tester.pump(const Duration(seconds: 1));
      await contact.up();
      expect(events, ['cancel']);
    },
  );
}
