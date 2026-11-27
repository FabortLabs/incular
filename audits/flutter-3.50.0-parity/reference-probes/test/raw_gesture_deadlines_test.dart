import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('An idle raw long press starts exactly at 500ms', (
    WidgetTester tester,
  ) async {
    final events = <String>[];
    await tester.pumpWidget(
      Center(
        child: RawGestureDetector(
          behavior: HitTestBehavior.opaque,
          gestures: <Type, GestureRecognizerFactory>{
            LongPressGestureRecognizer:
                GestureRecognizerFactoryWithHandlers<
                  LongPressGestureRecognizer
                >(LongPressGestureRecognizer.new, (hold) {
                  hold.onLongPressStart = (_) => events.add('start');
                  hold.onLongPressEnd = (_) => events.add('end');
                }),
          },
          child: const SizedBox(width: 100, height: 100),
        ),
      ),
    );
    final contact = await tester.startGesture(
      tester.getCenter(find.byType(RawGestureDetector)),
    );
    await tester.pump(const Duration(milliseconds: 499));
    expect(events, isEmpty);
    await tester.pump(const Duration(milliseconds: 1));
    expect(events, ['start']);
    await contact.up();
    expect(events, ['start', 'end']);
  });

  testWidgets(
    'A later frame still resolves the earlier consecutive-tap timer',
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
                  >(LongPressGestureRecognizer.new, (hold) {
                    hold.onLongPressStart = (_) => events.add('hold');
                    hold.onLongPressCancel = () => events.add('cancel');
                  }),
              TapAndPanGestureRecognizer:
                  GestureRecognizerFactoryWithHandlers<
                    TapAndPanGestureRecognizer
                  >(TapAndPanGestureRecognizer.new, (tap) {
                    tap.onTapDown = (details) =>
                        events.add('down#${details.consecutiveTapCount}');
                    tap.onTapUp = (details) =>
                        events.add('up#${details.consecutiveTapCount}');
                  }),
            },
            child: const SizedBox(width: 100, height: 100),
          ),
        ),
      );
      final target = find.byType(RawGestureDetector);
      await tester.tap(target);
      expect(events, ['cancel', 'down#1', 'up#1']);
      await tester.pump(const Duration(milliseconds: 150));
      final second = await tester.startGesture(tester.getCenter(target));
      await tester.pump(const Duration(milliseconds: 500));
      expect(events, ['cancel', 'down#1', 'up#1', 'down#2']);
      await second.up();
      expect(events.last, 'up#2');
    },
  );
}
