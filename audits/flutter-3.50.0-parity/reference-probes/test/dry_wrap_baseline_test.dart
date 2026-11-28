import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('dry Wrap’s up-direction baseline is offset from actual layout', (
    tester,
  ) async {
    for (final (direction, actual, speculative)
        in <(VerticalDirection, double, double)>[
          (VerticalDirection.down, 30, 30),
          (VerticalDirection.up, 15, 82),
        ]) {
      await tester.pumpWidget(
        Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 50, maxHeight: 101),
            child: Baseline(
              baseline: 50,
              baselineType: TextBaseline.alphabetic,
              child: Wrap(
                key: ValueKey(direction),
                textDirection: TextDirection.ltr,
                verticalDirection: direction,
                runSpacing: 7,
                children: const [
                  Text(
                    'X',
                    textDirection: TextDirection.ltr,
                    style: TextStyle(fontFamily: 'FlutterTest', fontSize: 40),
                  ),
                  Text(
                    'X',
                    textDirection: TextDirection.ltr,
                    style: TextStyle(fontFamily: 'FlutterTest', fontSize: 20),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      final box = tester.renderObject<RenderBox>(find.byType(Wrap));
      const dry = BoxConstraints(maxWidth: 50, maxHeight: 100);
      expect(box.getDryLayout(dry), const Size(40, 67));
      // Keep this bound distinct from the live layout. In Flutter 3.50.0,
      // a cached actual baseline at identical bounds can assert in this query.
      expect(box.getDryBaseline(dry, TextBaseline.alphabetic), speculative);
      expect(
        tester.getTopLeft(find.byType(Wrap)).dy -
            tester.getTopLeft(find.byType(Baseline)).dy,
        50 - actual,
      );
    }
  });
}
