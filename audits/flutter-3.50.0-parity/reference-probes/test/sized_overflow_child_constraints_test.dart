import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('SizedOverflowBox passes incoming constraints to its child', (
    WidgetTester tester,
  ) async {
    const Key box = ValueKey('box');
    const Key child = ValueKey('child');
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: SizedOverflowBox(
            key: box,
            size: Size(100, 100),
            child: SizedBox(key: child, width: 200, height: 150),
          ),
        ),
      ),
    );
    expect(
      tester.getRect(find.byKey(box)),
      const Rect.fromLTWH(350, 250, 100, 100),
    );
    expect(
      tester.getRect(find.byKey(child)),
      const Rect.fromLTWH(300, 225, 200, 150),
    );
  });
}
