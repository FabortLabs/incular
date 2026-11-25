import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'logical transformed overflow follows ambient and explicit direction',
    (tester) async {
      const proxy = Key('proxy');
      const child = Key('child');
      for (final (ambient, explicit, x) in [
        (TextDirection.ltr, null, 0.0),
        (TextDirection.rtl, null, -100.0),
        (TextDirection.rtl, TextDirection.ltr, 0.0),
      ]) {
        await tester.pumpWidget(
          Directionality(
            textDirection: ambient,
            child: Center(
              child: SizedBox.square(
                dimension: 100,
                child: ConstraintsTransformBox(
                  key: proxy,
                  constraintsTransform:
                      ConstraintsTransformBox.maxWidthUnconstrained,
                  alignment: AlignmentDirectional.topStart,
                  textDirection: explicit,
                  clipBehavior: Clip.hardEdge,
                  child: const SizedBox(key: child, width: 200, height: 50),
                ),
              ),
            ),
          ),
        );
        expect(
          tester.getTopLeft(find.byKey(child)) -
              tester.getTopLeft(find.byKey(proxy)),
          Offset(x, 0),
        );
      }
    },
  );
}
