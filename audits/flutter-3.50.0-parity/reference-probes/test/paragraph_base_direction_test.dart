import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'changing paragraph direction reorders its unchanged source runs',
    (WidgetTester tester) async {
      final key = GlobalKey();
      Widget view(TextDirection ambient, [TextDirection? explicit]) =>
          Directionality(
            textDirection: ambient,
            child: Align(
              alignment: Alignment.topLeft,
              child: Text(
                'אבג xyz',
                key: key,
                textAlign: TextAlign.left,
                textDirection: explicit,
              ),
            ),
          );

      double left(RenderParagraph text, int start, int end) => text
          .getBoxesForSelection(
            TextSelection(baseOffset: start, extentOffset: end),
          )
          .map((box) => box.left)
          .reduce((a, b) => a < b ? a : b);

      await tester.pumpWidget(view(TextDirection.rtl));
      final retained = tester.renderObject<RenderParagraph>(find.byKey(key));
      expect(left(retained, 4, 7), lessThan(left(retained, 0, 3)));

      await tester.pumpWidget(view(TextDirection.ltr));
      expect(tester.renderObject(find.byKey(key)), same(retained));
      expect(left(retained, 0, 3), lessThan(left(retained, 4, 7)));

      await tester.pumpWidget(view(TextDirection.ltr, TextDirection.rtl));
      expect(left(retained, 4, 7), lessThan(left(retained, 0, 3)));
    },
  );
}
