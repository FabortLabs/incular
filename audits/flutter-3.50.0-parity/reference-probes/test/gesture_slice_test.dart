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
    Future<(Offset, double)> run(bool synthetic, int pointer) async {
      final ends = <double>[];
      final updates = <Offset>[];
      await t.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onScaleStart: (_) {},
            onScaleUpdate: (d) => updates.add(d.focalPoint),
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
      return (updates.last, ends.single);
    }

    final (ordinaryFocal, ordinaryVelocity) = await run(false, 1);
    final (syntheticFocal, syntheticVelocity) = await run(true, 2);
    expect(ordinaryFocal, const Offset(180, 100));
    expect(syntheticFocal, ordinaryFocal);
    expect(ordinaryVelocity, greaterThan(1000));
    expect(syntheticVelocity, 0);
  });

  testWidgets(
    'mixed scale samples keep geometry but ignore synthesized velocity',
    (tester) async {
      final rawFlags = <bool>[];
      final focalPoints = <Offset>[];
      final releases = <Offset>[];
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Listener(
            onPointerMove: (event) => rawFlags.add(event.synthesized),
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onScaleStart: (_) {},
              onScaleUpdate: (details) => focalPoints.add(details.focalPoint),
              onScaleEnd: (details) =>
                  releases.add(details.velocity.pixelsPerSecond),
              child: const SizedBox.expand(),
            ),
          ),
        ),
      );
      const origin = Offset(100, 100);
      const motion = [
        (20.0, false),
        (145.0, true),
        (60.0, false),
        (-20.0, true),
        (100.0, false),
      ];
      tester.binding.handlePointerEvent(
        const PointerDownEvent(pointer: 1, position: origin),
      );
      var previous = origin;
      for (final (index, (relativeX, synthetic)) in motion.indexed) {
        final position = origin + Offset(relativeX, 0);
        tester.binding.handlePointerEvent(
          PointerMoveEvent(
            pointer: 1,
            position: position,
            delta: position - previous,
            timeStamp: Duration(milliseconds: (index + 1) * 10),
            synthesized: synthetic,
          ),
        );
        previous = position;
      }
      tester.binding.handlePointerEvent(
        PointerUpEvent(
          pointer: 1,
          position: previous,
          timeStamp: const Duration(milliseconds: 55),
        ),
      );
      expect(rawFlags, [false, true, false, true, false]);
      expect(
        focalPoints,
        containsAll([const Offset(245, 100), const Offset(80, 100)]),
      );
      expect(focalPoints.last, const Offset(200, 100));
      expect(releases.single.dx, closeTo(2000, 0.01));
      expect(releases.single.dy, closeTo(0, 0.01));
    },
  );
}
