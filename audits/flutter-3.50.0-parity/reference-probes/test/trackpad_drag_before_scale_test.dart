import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('early horizontal trackpad drag wins even if zoom follows', (tester) async {
    final starts = <DragStartDetails>[];
    final updates = <DragUpdateDetails>[];
    final ends = <DragEndDetails>[];
    final scale = <String>[];
    await tester.pumpWidget(
      GestureDetector(
        behavior: HitTestBehavior.opaque,
        onScaleStart: (_) => scale.add('start'),
        onScaleUpdate: (_) => scale.add('update'),
        onScaleEnd: (_) => scale.add('end'),
        onHorizontalDragStart: starts.add,
        onHorizontalDragUpdate: updates.add,
        onHorizontalDragEnd: ends.add,
        child: const SizedBox.expand(),
      ),
    );

    const origin = Offset(30, 30);
    final gesture = await tester.startGesture(origin, kind: PointerDeviceKind.trackpad);
    await gesture.panZoomUpdate(origin, pan: const Offset(18, 0));
    expect(starts, isEmpty);
    await gesture.panZoomUpdate(origin, pan: const Offset(20, 0));
    expect(starts.single.globalPosition, const Offset(50, 30));
    expect(starts.single.kind, PointerDeviceKind.trackpad);
    expect(updates, isEmpty);

    await gesture.panZoomUpdate(
      const Offset(32, 40),
      pan: const Offset(70, 5),
      scale: 2,
      rotation: 1,
    );
    expect(scale, isEmpty);
    expect(updates.single.delta, const Offset(50, 0));
    expect(updates.single.primaryDelta, 50);
    expect(updates.single.globalPosition, const Offset(102, 45));
    await gesture.panZoomEnd();
    expect(ends, hasLength(1));
    expect(scale, isEmpty);
  });
}
