import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('rotated local horizontal drag competes in the correct axis', (
    t,
  ) async {
    Future<int> run(double angle, Offset delta) async {
      const key = ValueKey('target');
      var starts = 0;
      await t.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: GestureDetector(
              onTap: () {},
              child: Transform.rotate(
                angle: angle,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onHorizontalDragStart: (_) {
                    starts++;
                  },
                  child: const SizedBox(key: key, width: 200, height: 200),
                ),
              ),
            ),
          ),
        ),
      );
      final p =
          t.getTopLeft(find.byKey(key)) +
          (angle == 0 ? const Offset(50, 50) : const Offset(-50, 50));
      final g = await t.startGesture(p);
      await g.moveBy(delta);
      final n = starts;
      await g.cancel();
      return n;
    }

    expect(await run(0, const Offset(20, 0)), 1);
    expect(await run(math.pi / 2, const Offset(0, 17)), 0);
    expect(await run(math.pi / 2, const Offset(0, 20)), 1);
  });
  testWidgets('synthesized moves do not feed scale velocity', (t) async {
    Future<double> run(bool synthetic, int pointer) async {
      final ends = <double>[];
      await t.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onScaleStart: (_) {},
            onScaleEnd: (d) => ends.add(d.velocity.pixelsPerSecond.dx),
            child: const SizedBox.expand(),
          ),
        ),
      );
      const start = Offset(100, 100);
      t.binding.handlePointerEvent(
        PointerDownEvent(pointer: pointer, position: start),
      );
      for (var k = 1; k <= 4; k++) {
        t.binding.handlePointerEvent(
          PointerMoveEvent(
            pointer: pointer,
            position: start + Offset(20.0 * k, 0),
            delta: const Offset(20, 0),
            timeStamp: Duration(milliseconds: 10 * k),
            synthesized: synthetic,
          ),
        );
      }
      t.binding.handlePointerEvent(
        PointerUpEvent(
          pointer: pointer,
          position: start + const Offset(80, 0),
          timeStamp: const Duration(milliseconds: 45),
        ),
      );
      return ends.single;
    }

    expect(await run(false, 1), greaterThan(1000));
    expect(await run(true, 2), 0);
  });
}
