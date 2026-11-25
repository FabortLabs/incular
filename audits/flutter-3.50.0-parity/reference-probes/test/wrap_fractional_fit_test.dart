import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final axis in Axis.values) {
    testWidgets('fractional fit and genuine overflow on $axis', (tester) async {
      const first = ValueKey('first');
      const second = ValueKey('second');
      for (final (spacing, secondSize) in [(0.0, 10.3), (0.1, 10.2)]) {
        Future<double> crossOffset(double limit) async {
          final horizontal = axis == Axis.horizontal;
          Widget child(Key key, double main) => SizedBox(
            key: key,
            width: horizontal ? main : 8,
            height: horizontal ? 8 : main,
          );
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: Center(
                child: SizedBox(
                  width: horizontal ? limit : 80,
                  height: horizontal ? 80 : limit,
                  child: Wrap(
                    direction: axis,
                    spacing: spacing,
                    children: [child(first, 10.1), child(second, secondSize)],
                  ),
                ),
              ),
            ),
          );
          final delta =
              tester.getTopLeft(find.byKey(second)) -
              tester.getTopLeft(find.byKey(first));
          return horizontal ? delta.dy : delta.dx;
        }

        expect(await crossOffset(20.4), 0);
        expect(await crossOffset(20.399998), 8);
      }
    });
  }
}
