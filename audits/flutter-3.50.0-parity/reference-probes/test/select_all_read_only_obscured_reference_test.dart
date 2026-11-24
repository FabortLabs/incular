import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Ctrl+A respects read-only obscured Select All gate', (
    tester,
  ) async {
    Future<TextSelection> check(bool obscure) async {
      final controller = TextEditingController.fromValue(
        const TextEditingValue(
          text: 'blah blah',
          selection: TextSelection.collapsed(offset: 0),
        ),
      );
      final focus = FocusNode();
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: MediaQuery(
            data: const MediaQueryData(),
            child: DefaultTextEditingShortcuts(
              child: EditableText(
                controller: controller,
                focusNode: focus,
                style: const TextStyle(fontSize: 14),
                cursorColor: const Color(0xff000000),
                backgroundCursorColor: const Color(0xff000000),
                readOnly: true,
                obscureText: obscure,
                autofocus: true,
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      expect(focus.hasFocus, isTrue);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await tester.pump();
      final result = controller.selection;
      await tester.pumpWidget(const SizedBox.shrink());
      controller.dispose();
      focus.dispose();
      return result;
    }

    expect(
      await check(false),
      const TextSelection(baseOffset: 0, extentOffset: 9),
    );
    expect(await check(true), const TextSelection.collapsed(offset: 0));
  }, variant: TargetPlatformVariant.only(TargetPlatform.windows));
}
