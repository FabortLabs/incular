import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Future<void> delete(
  WidgetTester tester, {
  required String text,
  required TextSelection selection,
  required String expectedText,
  required int expectedCaret,
  LogicalKeyboardKey key = LogicalKeyboardKey.delete,
  bool obscured = false,
}) async {
  final controller = TextEditingController(text: text)..selection = selection;
  final focus = FocusNode();
  addTearDown(controller.dispose);
  addTearDown(focus.dispose);
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: DefaultTextEditingShortcuts(
        child: EditableText(
          controller: controller,
          focusNode: focus,
          autofocus: true,
          obscureText: obscured,
          style: const TextStyle(fontSize: 10),
          cursorColor: const Color(0xff000000),
          backgroundCursorColor: const Color(0xff999999),
        ),
      ),
    ),
  );
  await tester.pump();
  expect(focus.hasFocus, isTrue);
  await tester.sendKeyEvent(key);
  await tester.pump();
  expect(controller.text, expectedText);
  expect(controller.selection, TextSelection.collapsed(offset: expectedCaret));
  expect(focus.hasFocus, isTrue);
}

void main() {
  for (final reversed in [false, true]) {
    testWidgets(
      'delete expands partially selected graphemes (reversed: $reversed)',
      (tester) async {
        await delete(
          tester,
          text: 'Ce\u0301o\u0308D',
          selection: TextSelection(
            baseOffset: reversed ? 4 : 2,
            extentOffset: reversed ? 2 : 4,
          ),
          expectedText: 'CD',
          expectedCaret: 1,
        );
      },
    );
  }
  for (final key in [LogicalKeyboardKey.delete, LogicalKeyboardKey.backspace]) {
    testWidgets(
      '${key.keyLabel} removes the full grapheme containing the caret',
      (tester) async {
        await delete(
          tester,
          text: 'Ce\u0301D',
          selection: const TextSelection.collapsed(
            offset: 2,
            affinity: TextAffinity.upstream,
          ),
          expectedText: 'CD',
          expectedCaret: 1,
          key: key,
        );
      },
    );
  }
  testWidgets('obscured text deletes only the selected code points', (
    tester,
  ) async {
    await delete(
      tester,
      text: 'Ce\u0301D',
      selection: const TextSelection(baseOffset: 2, extentOffset: 3),
      expectedText: 'CeD',
      expectedCaret: 2,
      obscured: true,
    );
  });
}
