import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Childless FractionallySizedBox still reserves bounded fractions',
    (tester) async {
      const key = ValueKey('box');
      await tester.pumpWidget(
        View(
          view: tester.view,
          child: const Center(
            child: FractionallySizedBox(
              key: key,
              widthFactor: .5,
              heightFactor: .25,
              alignment: AlignmentDirectional.topEnd,
            ),
          ),
        ),
        wrapWithView: false,
      );
      // No user-provided Directionality and no child from which to infer size.
      expect(tester.takeException(), isNull);
      expect(
        tester.getRect(find.byKey(key)),
        const Rect.fromLTWH(200, 225, 400, 150),
      );
    },
  );

  testWidgets(
    'Childless AnimatedFractionallySizedBox interpolates the tight factors',
    (tester) async {
      const key = ValueKey('animated');
      Widget scene(double factor) => Center(
        child: SizedBox.square(
          dimension: 200,
          child: Center(
            child: AnimatedFractionallySizedBox(
              key: key,
              duration: const Duration(milliseconds: 200),
              widthFactor: factor,
              heightFactor: factor,
            ),
          ),
        ),
      );
      await tester.pumpWidget(scene(.5));
      expect(tester.getSize(find.byKey(key)), const Size(100, 100));
      await tester.pumpWidget(scene(1));
      await tester.pump(const Duration(milliseconds: 100));
      expect(tester.getSize(find.byKey(key)), const Size(150, 150));
      await tester.pump(const Duration(milliseconds: 100));
      expect(tester.getSize(find.byKey(key)), const Size(200, 200));
    },
  );
}
