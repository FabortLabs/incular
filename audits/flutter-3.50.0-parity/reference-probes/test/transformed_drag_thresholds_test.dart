import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const key = ValueKey<String>('drag');

  Widget scene(
    double xScale, {
    DragStartBehavior behavior = DragStartBehavior.start,
    GestureDragStartCallback? start,
    GestureDragUpdateCallback? update,
  }) => Center(
    child: GestureDetector(
      onTap: () {},
      child: Transform.scale(
        scaleX: xScale,
        scaleY: 0.5,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          dragStartBehavior: behavior,
          onHorizontalDragStart: start,
          onHorizontalDragUpdate: update,
          child: const SizedBox(key: key, width: 200, height: 200),
        ),
      ),
    ),
  );

  testWidgets('Drag slop stays in physical units under affine scale', (
    WidgetTester tester,
  ) async {
    for (final scale in [4.0, 0.25, -4.0]) {
      var starts = 0;
      await tester.pumpWidget(scene(scale, start: (_) => starts += 1));
      final origin = tester.getCenter(find.byKey(key));
      final drag = await tester.startGesture(origin);
      try {
        await drag.moveBy(const Offset(17, 0));
        expect(starts, 0);
        await drag.moveBy(const Offset(1, 0));
        expect(starts, 0);
        await drag.moveBy(const Offset(1, 0));
        expect(starts, 1);
      } finally {
        await drag.cancel();
      }
    }
  });

  testWidgets(
    'Down behavior reports the corrected axis-projected first update',
    (WidgetTester tester) async {
      final starts = <DragStartDetails>[];
      final updates = <DragUpdateDetails>[];
      await tester.pumpWidget(
        scene(
          4,
          behavior: DragStartBehavior.down,
          start: starts.add,
          update: updates.add,
        ),
      );
      final origin = tester.getCenter(find.byKey(key));
      final drag = await tester.startGesture(origin);
      try {
        await drag.moveBy(const Offset(20, 10));
        expect(starts.single.globalPosition, origin);
        expect(starts.single.localPosition, const Offset(100, 100));
        expect(updates.single.delta, const Offset(5, 0));
        expect(updates.single.primaryDelta, 5);
        expect(updates.single.globalPosition, origin + const Offset(20, 0));
        expect(updates.single.localPosition, const Offset(105, 100));

        await drag.moveBy(const Offset(8, 5));
        expect(updates.last.delta, const Offset(2, 0));
        expect(updates.last.globalPosition, origin + const Offset(28, 15));
        expect(updates.last.localPosition, const Offset(107, 130));
      } finally {
        await drag.cancel();
      }
    },
  );
}
