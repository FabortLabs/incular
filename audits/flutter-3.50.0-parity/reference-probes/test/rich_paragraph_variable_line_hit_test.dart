import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Rich paragraph hit testing uses each actual line height', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: RichText(
            text: const TextSpan(
              text: 'a\n',
              style: TextStyle(fontSize: 10, height: 1),
              children: [
                TextSpan(text: 'bright\n', style: TextStyle(fontSize: 30)),
                TextSpan(text: 'last'),
              ],
            ),
          ),
        ),
      ),
    );
    final paragraph = tester.renderObject<RenderParagraph>(
      find.byType(RichText),
    );
    expect(paragraph.getPositionForOffset(const Offset(25, 25)).offset, 3);
    expect(paragraph.getPositionForOffset(const Offset(21, 5)).offset, 1);
    expect(paragraph.getPositionForOffset(const Offset(21, 45)).offset, 11);
  });
}
