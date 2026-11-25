import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('An arena-owned drag waits for strict slop when requested', (
    WidgetTester tester,
  ) async {
    final starts = <DragStartDetails>[];
    final updates = <DragUpdateDetails>[];
    final ends = <DragEndDetails>[];
    await tester.pumpWidget(
      RawGestureDetector(
        behavior: HitTestBehavior.opaque,
        gestures: <Type, GestureRecognizerFactory>{
          VerticalDragGestureRecognizer:
              GestureRecognizerFactoryWithHandlers<
                VerticalDragGestureRecognizer
              >(VerticalDragGestureRecognizer.new, (recognizer) {
                recognizer
                  ..onlyAcceptDragOnThreshold = true
                  ..onStart = starts.add
                  ..onUpdate = updates.add
                  ..onEnd = ends.add;
              }),
        },
        child: const SizedBox.expand(),
      ),
    );

    const origin = Offset(10, 10);
    final drag = await tester.startGesture(origin);
    await tester.pump();
    expect(starts, isEmpty);
    await drag.moveBy(const Offset(0, 17));
    expect(starts, isEmpty);
    await drag.moveBy(const Offset(0, 1));
    expect(starts, isEmpty);
    await drag.moveBy(const Offset(0, 1));
    expect(starts.single.globalPosition, const Offset(10, 29));
    expect(updates, isEmpty);
    await drag.up();
    expect(starts, hasLength(1));
    expect(updates, isEmpty);
    expect(ends, hasLength(1));
  });
}
