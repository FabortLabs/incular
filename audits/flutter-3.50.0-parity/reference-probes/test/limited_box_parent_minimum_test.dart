import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'LimitedBox cannot lower the parent minimum on an unbounded axis',
    (tester) async {
      Future<Rect> layout({Widget? child}) async {
        const key = ValueKey('limited');
        await tester.pumpWidget(
          const Directionality(
            textDirection: TextDirection.ltr,
            child: SizedBox.shrink(),
          ),
        );
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: OverflowBox(
              minWidth: 150,
              maxWidth: double.infinity,
              minHeight: 250,
              maxHeight: double.infinity,
              child: LimitedBox(
                key: key,
                maxWidth: 100,
                maxHeight: 200,
                child: child,
              ),
            ),
          ),
        );
        expect(tester.takeException(), isNull);
        return tester.getRect(find.byKey(key));
      }

      const bounds = Rect.fromLTWH(325, 175, 150, 250);
      expect(
        await layout(child: const SizedBox(width: 300, height: 400)),
        bounds,
      );
      expect(await layout(), bounds);
    },
  );
}
