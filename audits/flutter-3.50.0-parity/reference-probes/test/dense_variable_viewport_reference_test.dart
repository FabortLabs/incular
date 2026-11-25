import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('short natural rows fill a 600px viewport on the first frame', (
    WidgetTester tester,
  ) async {
    int built = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: ListView.builder(
          cacheExtent: 0,
          itemCount: 1000000,
          itemBuilder: (_, i) {
            built++;
            return SizedBox(key: ValueKey<int>(i), height: 12.3);
          },
        ),
      ),
    );
    for (int i = 0; i < 49; i++) {
      final rect = tester.getRect(find.byKey(ValueKey<int>(i)));
      expect(rect.top, closeTo(i * 12.3, 0.000001));
      expect(rect.top, lessThan(600));
      if (i == 48) {
        expect(rect.bottom, greaterThan(600));
      }
    }
    expect(
      find.byKey(const ValueKey<int>(49), skipOffstage: false),
      findsNothing,
    );
    expect(built, 49);
  });
}
