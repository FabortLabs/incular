import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('directional box border insets the correct physical edges', (tester) async {
    const parent = ValueKey('box');
    const content = ValueKey('content');
    for (final (TextDirection direction, double childX) in [
      (TextDirection.ltr, 6.0),
      (TextDirection.rtl, 2.0),
    ]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: direction,
          child: Align(
            alignment: Alignment.topLeft,
            child: Container(
              key: parent,
              width: 40,
              height: 20,
              decoration: const BoxDecoration(
                border: BorderDirectional(
                  start: BorderSide(color: Color(0xFFFF0000), width: 6),
                  end: BorderSide(color: Color(0xFF0000FF), width: 2),
                ),
              ),
              child: const ColoredBox(key: content, color: Color(0xFF00FF00)),
            ),
          ),
        ),
      );
      final outer = tester.getRect(find.byKey(parent));
      final inner = tester.getRect(find.byKey(content));
      expect(outer.size, const Size(40, 20));
      expect(inner.shift(-outer.topLeft), Rect.fromLTWH(childX, 0, 32, 20));
    }
  });
}
