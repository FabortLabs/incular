import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a mounted editor switches between full and natural line width', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController(text: 'A');
    final FocusNode focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);

    Future<Size> layout({required bool forceLine, double minimum = 0, double maximum = 500}) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: minimum, maxWidth: maximum),
              child: EditableText(
                controller: controller,
                focusNode: focus,
                style: const TextStyle(fontSize: 10, height: 1),
                cursorColor: const Color(0xffffffff),
                backgroundCursorColor: const Color(0xff222222),
                cursorWidth: 2,
                forceLine: forceLine,
              ),
            ),
          ),
        ),
      );
      return tester.getSize(find.byType(EditableText));
    }

    expect(await layout(forceLine: true, minimum: 100), const Size(500, 10));
    final State<EditableText> state = tester.state(find.byType(EditableText));
    expect(await layout(forceLine: false, minimum: 100), const Size(103, 10));
    expect(await layout(forceLine: false), const Size(13, 10));
    expect(await layout(forceLine: false, minimum: 130, maximum: 130), const Size(130, 10));
    expect(await layout(forceLine: true, minimum: 100), const Size(500, 10));
    expect(tester.state(find.byType(EditableText)), same(state));
    expect(controller.text, 'A');
  });

  testWidgets('parent and longest-line bases differ after a wrap', (WidgetTester tester) async {
    final TextEditingController controller = TextEditingController(text: 'AA BB CC');
    final FocusNode focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    Future<Size> measure(TextWidthBasis basis, {bool forced = false}) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 50),
              child: EditableText(
                controller: controller,
                focusNode: focus,
                style: const TextStyle(fontSize: 10, height: 1),
                cursorColor: const Color(0xffffffff),
                backgroundCursorColor: const Color(0xff222222),
                cursorWidth: 2,
                maxLines: null,
                forceLine: forced,
                textWidthBasis: basis,
              ),
            ),
          ),
        ),
      );
      return tester.getSize(find.byType(EditableText));
    }

    expect(await measure(TextWidthBasis.parent), const Size(50, 30));
    expect(await measure(TextWidthBasis.longestLine), const Size(23, 30));
    expect(await measure(TextWidthBasis.longestLine, forced: true), const Size(50, 30));
    expect(controller.text, 'AA BB CC');
  });
}
