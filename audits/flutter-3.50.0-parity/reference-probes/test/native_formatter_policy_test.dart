import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget editor(
  TextEditingController controller,
  FocusNode focus,
  TextInputFormatter formatter, {
  int? maxLines = 1,
}) => Directionality(
  textDirection: TextDirection.ltr,
  child: EditableText(
    controller: controller,
    focusNode: focus,
    style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
    cursorColor: const Color(0xff000000),
    backgroundCursorColor: const Color(0xff808080),
    maxLines: maxLines,
    inputFormatters: [formatter],
  ),
);

void main() {
  testWidgets('selection and marked-range changes do not rerun formatters', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'abc');
    final node = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(node.dispose);
    var calls = 0;
    await tester.pumpWidget(
      editor(controller, node, TextInputFormatter.withFunction((old, next) {
        calls++;
        return next;
      })),
    );
    node.requestFocus();
    await tester.pump();
    Future<void> update(
      String text,
      int offset, [
      TextRange composing = TextRange.empty,
    ]) async {
      tester.testTextInput.updateEditingValue(
        TextEditingValue(
          text: text,
          selection: TextSelection.collapsed(offset: offset),
          composing: composing,
        ),
      );
      await tester.pump();
    }

    await update('abc', 1);
    expect(controller.selection.baseOffset, 1);
    expect(calls, 0);
    await update('axbc', 2);
    expect(calls, 1);
    await update('axbc', 2, const TextRange(start: 0, end: 2));
    await update('axbc', 2, const TextRange(start: 1, end: 2));
    expect(controller.value.composing, const TextRange(start: 1, end: 2));
    expect(calls, 1);
    await update('axbc', 2);
    expect(controller.value.composing, TextRange.empty);
    expect(calls, 2);
  });

  testWidgets('single line filtering precedes caller formatters', (tester) async {
    for (final multiline in [false, true]) {
      final controller = TextEditingController();
      final node = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(node.dispose);
      final visited = <String>[];
      await tester.pumpWidget(
        editor(
          controller,
          node,
          TextInputFormatter.withFunction((old, next) {
            visited.add(next.text);
            return next;
          }),
          maxLines: multiline ? null : 1,
        ),
      );
      node.requestFocus();
      await tester.pump();
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'a😀\nb',
          selection: TextSelection.collapsed(offset: 4),
        ),
      );
      await tester.pump();
      final expected = multiline ? 'a😀\nb' : 'a😀b';
      expect(visited, [expected]);
      expect(controller.text, expected);
      expect(controller.selection.baseOffset, multiline ? 4 : 3);
      await tester.pumpWidget(const SizedBox.shrink());
    }
  });
}
