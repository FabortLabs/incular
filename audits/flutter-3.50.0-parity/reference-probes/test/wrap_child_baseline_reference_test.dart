import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Wrap passes through the alphabetic text baseline', (
    tester,
  ) async {
    const probe = ValueKey('baseline');
    const glyph = Text(
      'X',
      textDirection: TextDirection.ltr,
      style: TextStyle(fontSize: 100, height: 1),
    );
    Future<Size> beside(Widget child) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: Baseline(
              key: probe,
              baseline: 175,
              baselineType: TextBaseline.alphabetic,
              child: child,
            ),
          ),
        ),
      );
      return tester.getSize(find.byKey(probe));
    }

    final direct = await beside(glyph);
    expect(direct.height, greaterThan(175));
    expect(await beside(const Wrap(children: [glyph])), direct);
  });
}
