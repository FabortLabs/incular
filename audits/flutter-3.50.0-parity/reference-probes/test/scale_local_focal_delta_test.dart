import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('scaled gestures keep focal positions global but delta local', (
    WidgetTester tester,
  ) async {
    const target = ValueKey('target');
    final starts = <ScaleStartDetails>[];
    final updates = <ScaleUpdateDetails>[];
    await tester.pumpWidget(
      Center(
        child: Transform.scale(
          scale: 2,
          child: GestureDetector(
            key: target,
            behavior: HitTestBehavior.opaque,
            onScaleStart: starts.add,
            onScaleUpdate: updates.add,
            child: const SizedBox(width: 100, height: 100),
          ),
        ),
      ),
    );
    final origin = tester.getTopLeft(find.byKey(target));
    final down = origin + const Offset(40, 40);
    final pointer = await tester.startGesture(down);
    await tester.pump();
    expect(starts.single.focalPoint, down);
    expect(starts.single.localFocalPoint, const Offset(20, 20));
    expect(updates, isEmpty);
    await pointer.moveBy(const Offset(40, 20));
    expect(updates.single.focalPoint, down + const Offset(40, 20));
    expect(updates.single.localFocalPoint, const Offset(40, 30));
    expect(updates.single.focalPointDelta, const Offset(20, 10));
    await pointer.up();
  });
}
