import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final transformHits in [true, false]) {
    testWidgets(
      'retained listener after moving and rescaling: $transformHits',
      (tester) async {
        const key = ValueKey('listener');
        final trace = <(Offset, Offset)>[];
        void record(PointerEvent event) {
          trace.add((event.localPosition, event.localDelta));
        }

        Widget scene(double left, double scale) {
          final listener = Listener(
            key: key,
            behavior: HitTestBehavior.opaque,
            onPointerDown: record,
            onPointerMove: record,
            onPointerUp: record,
            onPointerHover: record,
            child: const SizedBox(width: 40, height: 40),
          );
          final child = transformHits
              ? Transform.scale(
                  alignment: Alignment.topLeft,
                  scale: scale,
                  child: listener,
                )
              : FractionalTranslation(
                  translation: Offset(scale / 2, 0),
                  transformHitTests: false,
                  child: listener,
                );
          return Directionality(
            textDirection: TextDirection.ltr,
            child: Stack(
              children: [Positioned(left: left, top: 0, child: child)],
            ),
          );
        }

        await tester.pumpWidget(scene(0, 2));
        final first = tester.element(find.byKey(key));
        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.down(const Offset(20, 20));
        await tester.pumpWidget(scene(40, 4));
        expect(tester.element(find.byKey(key)), same(first));
        await mouse.moveTo(const Offset(30, 28));
        await mouse.up();
        await mouse.moveTo(const Offset(60, 20));
        await mouse.down(const Offset(60, 20));
        expect(
          trace,
          transformHits
              ? const [
                  (Offset(10, 10), Offset.zero),
                  (Offset(15, 14), Offset(5, 4)),
                  (Offset(15, 14), Offset.zero),
                  (Offset(5, 5), Offset(7.5, -2)),
                  (Offset(5, 5), Offset.zero),
                ]
              : const [
                  (Offset(20, 20), Offset.zero),
                  (Offset(30, 28), Offset(10, 8)),
                  (Offset(30, 28), Offset.zero),
                  (Offset(20, 20), Offset(30, -8)),
                  (Offset(20, 20), Offset.zero),
                ],
        );
        await mouse.up();
        await mouse.removePointer();
      },
    );
  }
}
