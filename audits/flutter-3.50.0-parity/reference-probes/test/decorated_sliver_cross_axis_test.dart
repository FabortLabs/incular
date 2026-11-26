import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a constrained sliver reports its cross extent to its decorator',
    (tester) async {
      const paintKey = ValueKey('paint');
      const childKey = ValueKey('child');
      for (final axis in Axis.values) {
        for (final constraintInside in [true, false]) {
          const child = SliverToBoxAdapter(
            child: SizedBox(key: childKey, width: 100, height: 100),
          );
          Widget constrain(Widget sliver) =>
              SliverConstrainedCrossAxis(maxExtent: 120, sliver: sliver);
          Widget decorate(Widget sliver) => DecoratedSliver(
            key: paintKey,
            decoration: BoxDecoration(border: Border.all()),
            sliver: sliver,
          );
          final sliver = constraintInside
              ? decorate(constrain(child))
              : constrain(decorate(child));
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: 300,
                  height: 300,
                  child: CustomScrollView(
                    scrollDirection: axis,
                    slivers: [sliver],
                  ),
                ),
              ),
            ),
          );
          final expected = axis == Axis.vertical
              ? const Size(120, 100)
              : const Size(100, 120);
          expect(tester.getSize(find.byKey(childKey)), expected);
          expect(
            find.byKey(paintKey),
            paints..rect(
              rect: Rect.fromLTWH(
                0.5,
                0.5,
                expected.width - 1,
                expected.height - 1,
              ),
              style: PaintingStyle.stroke,
              strokeWidth: 1,
            ),
          );
        }
      }
    },
  );
}
