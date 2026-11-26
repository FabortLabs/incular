import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('caret margin across directions and line modes', (tester) async {
    tester.view.devicePixelRatio = 2;
    addTearDown(tester.view.resetDevicePixelRatio);
    final controller = TextEditingController();
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    for (final direction in [TextDirection.ltr, TextDirection.rtl]) {
      for (final maxLines in [1, 2]) {
        for (final align in [
          TextAlign.left,
          TextAlign.center,
          TextAlign.right,
        ]) {
          await tester.pumpWidget(
            Directionality(
              textDirection: direction,
              child: Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: 200,
                  height: 60,
                  child: EditableText(
                    controller: controller,
                    focusNode: focus,
                    style: const TextStyle(fontFamily: 'Ahem', fontSize: 14),
                    cursorColor: const Color(0xffffffff),
                    backgroundCursorColor: const Color(0xff808080),
                    textAlign: align,
                    maxLines: maxLines,
                  ),
                ),
              ),
            ),
          );
          final state = tester.state<EditableTextState>(
            find.byType(EditableText),
          );
          final rect = state.renderEditable.getLocalRectForCaret(
            const TextPosition(offset: 0),
          );
          expect(state.renderEditable.size.width, 200);
          expect(rect.width, 2);
          final expected = switch (align) {
            TextAlign.left => 0.0,
            TextAlign.center => 98.5,
            TextAlign.right => 197.0,
            _ => throw StateError('unexpected alignment'),
          };
          expect(
            rect.left,
            expected,
            reason: '${direction.name}, $maxLines lines, ${align.name}',
          );
        }
      }
    }
  });
}
