import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Only a focused writable visible editor underlines its composing range', (
    WidgetTester tester,
  ) async {
    for (final (readOnly, obscureText, autofocus) in [
      (false, false, true),
      (false, false, false),
      (true, false, true),
      (false, true, true),
    ]) {
      final controller = TextEditingController.fromValue(
        const TextEditingValue(
          text: 'text composing text',
          selection: TextSelection.collapsed(offset: 14),
          composing: TextRange(start: 5, end: 14),
        ),
      );
      final node = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(node.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: EditableText(
            key: ObjectKey(node),
            controller: controller,
            focusNode: node,
            readOnly: readOnly,
            obscureText: obscureText,
            autofocus: autofocus,
            style: const TextStyle(fontSize: 14),
            cursorColor: const Color(0xffffffff),
            backgroundCursorColor: const Color(0xff808080),
          ),
        ),
      );
      await tester.pump();
      expect(node.hasPrimaryFocus, autofocus);
      final state = tester.state<EditableTextState>(find.byType(EditableText));
      final span = state.renderEditable.text! as TextSpan;
      final runs = span.children?.whereType<TextSpan>() ?? const <TextSpan>[];
      final underlined = runs.where((run) => run.style?.decoration == TextDecoration.underline);
      expect(
        underlined.map((run) => run.text).toList(),
        autofocus && !readOnly && !obscureText ? ['composing'] : <String>[],
      );
      await tester.pumpWidget(const SizedBox());
    }
  });
}
