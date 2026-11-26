import 'package:flutter/foundation.dart' show TargetPlatform;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'seeded Windows undo history ignores Ctrl+Y and uses Ctrl+Shift+Z',
    (tester) async {
      final text = TextEditingController(text: 'abc');
      final focus = FocusNode();
      addTearDown(text.dispose);
      addTearDown(focus.dispose);
      await tester.pumpWidget(
        TestWidgetsApp(
          home: EditableText(
            controller: text,
            focusNode: focus,
            style: const TextStyle(),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xffeeeeee),
          ),
        ),
      );
      focus.requestFocus();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      text.value = const TextEditingValue(
        text: 'abcd',
        selection: TextSelection.collapsed(offset: 4),
      );
      await tester.pump(const Duration(milliseconds: 500));

      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
      expect(text.text, 'abc');
      await tester.sendKeyEvent(LogicalKeyboardKey.keyY);
      expect(text.text, 'abc');
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      expect(text.text, 'abcd');
      await tester.pumpWidget(const SizedBox.shrink());
    },
    variant: const TargetPlatformVariant({TargetPlatform.windows}),
  );
}
