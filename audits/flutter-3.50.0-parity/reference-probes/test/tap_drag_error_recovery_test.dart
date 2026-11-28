import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'A failing drag end is reported and the next tap starts a new series',
    (tester) async {
      final reports = <FlutterErrorDetails>[];
      final counts = <int>[];
      final previous = FlutterError.onError;
      FlutterError.onError = reports.add;
      try {
        await tester.pumpWidget(
          RawGestureDetector(
            behavior: HitTestBehavior.opaque,
            gestures: <Type, GestureRecognizerFactory>{
              TapAndPanGestureRecognizer:
                  GestureRecognizerFactoryWithHandlers<
                    TapAndPanGestureRecognizer
                  >(TapAndPanGestureRecognizer.new, (recognizer) {
                    recognizer.dragStartBehavior = DragStartBehavior.down;
                    recognizer.maxConsecutiveTap = 3;
                    recognizer.onTapDown = (details) {
                      counts.add(details.consecutiveTapCount);
                    };
                    recognizer.onDragEnd = (_) {
                      throw StateError('end callback failed');
                    };
                  }),
            },
            child: const SizedBox.expand(),
          ),
        );
        final drag = await tester.startGesture(const Offset(10, 10));
        await drag.moveBy(const Offset(40, 40));
        await drag.up();
        expect(counts, [1]);
        expect(reports, hasLength(1));
        expect(
          reports.single.exceptionAsString(),
          contains('end callback failed'),
        );
        expect(
          reports.single.context!.toDescription(),
          'while handling a gesture',
        );
        expect(
          reports.single.informationCollector!().any(
            (node) =>
                node.name == 'Handler' &&
                node.toDescription().contains('onDragEnd'),
          ),
          isTrue,
        );

        counts.clear();
        await tester.pump(const Duration(seconds: 1));
        await tester.tapAt(const Offset(10, 10));
        expect(counts, [1]);
        expect(reports, hasLength(1));
      } finally {
        FlutterError.onError = previous;
      }
    },
  );
}
