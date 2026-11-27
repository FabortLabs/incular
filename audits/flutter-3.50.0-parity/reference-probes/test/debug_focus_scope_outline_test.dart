import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('FocusScope does not add a second debug focus outline', (
    tester,
  ) async {
    debugPaintFocusBoxes = true;
    addTearDown(() => debugPaintFocusBoxes = false);

    const scopeKey = ValueKey('tested scope');
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: FocusScope(
          key: scopeKey,
          child: Focus(autofocus: true, child: SizedBox(width: 10, height: 10)),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final border = tester.widget<DecoratedBox>(
      find.descendant(
        of: find.byKey(scopeKey),
        matching: find.byType(DecoratedBox),
      ),
    );
    final side = ((border.decoration as BoxDecoration).border! as Border).top;
    expect(side.width, 3);
    expect(side.color, const Color(0xF000FF00));
    debugPaintFocusBoxes = false;
  });
}
