import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final fractional in [false, true]) {
    testWidgets(
      'Direction resolves the running alignment without restarting it ($fractional)',
      (tester) async {
        const target = ValueKey('target');
        const child = SizedBox(key: target, width: 50, height: 50);
        const duration = Duration(milliseconds: 200);
        var ends = 0;
        void completed() => ends++;
        Widget scene(TextDirection direction, AlignmentGeometry alignment) {
          final box = fractional
              ? AnimatedFractionallySizedBox(
                  alignment: alignment,
                  widthFactor: .25,
                  heightFactor: .25,
                  duration: duration,
                  onEnd: completed,
                  child: child,
                )
              : AnimatedAlign(
                  alignment: alignment,
                  duration: duration,
                  onEnd: completed,
                  child: child,
                );
          return Directionality(
            textDirection: direction,
            child: Align(
              alignment: Alignment.topLeft,
              child: SizedBox.square(dimension: 200, child: box),
            ),
          );
        }

        void check(double x, double y) {
          expect(
            tester.getRect(find.byKey(target)),
            rectMoreOrLessEquals(Rect.fromLTWH(x, y, 50, 50), epsilon: 1e-12),
          );
        }

        const start = Alignment.topLeft;
        const end = AlignmentDirectional.bottomEnd;
        await tester.pumpWidget(scene(TextDirection.ltr, start));
        check(0, 0);
        await tester.pumpWidget(scene(TextDirection.ltr, end));
        await tester.pump(const Duration(milliseconds: 100));
        check(75, 75);
        await tester.pumpWidget(scene(TextDirection.rtl, end));
        check(0, 75);
        expect(ends, 0);
        await tester.pump(const Duration(milliseconds: 50));
        check(0, 112.5);
        await tester.pump(const Duration(milliseconds: 50));
        check(0, 150);
        expect(ends, 0);
        await tester.pump(const Duration(milliseconds: 1));
        expect(ends, 1);
        await tester.pumpWidget(scene(TextDirection.ltr, end));
        check(150, 150);
        await tester.pump(duration);
        expect(ends, 1);
      },
    );
  }
}
