import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'desktop replacement selection and an explicit caret delete differently',
    (WidgetTester tester) async {
      final controller = TextEditingController();
      final node = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(node.dispose);
      await tester.pumpWidget(
        TestWidgetsApp(
          home: EditableText(
            controller: controller,
            focusNode: node,
            style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xff808080),
          ),
        ),
      );
      await tester.tap(find.byType(EditableText));
      await tester.pump();
      expect(node.hasPrimaryFocus, isTrue);
      controller.text = 'é👩‍💻';
      expect(
        controller.selection,
        const TextSelection(baseOffset: 0, extentOffset: 6),
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
      await tester.pump();
      expect(controller.text, isEmpty);

      controller.text = 'é👩‍💻';
      controller.selection = TextSelection.collapsed(
        offset: controller.text.length,
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
      await tester.pump();
      expect(controller.text, 'é');
      controller.selection = const TextSelection.collapsed(offset: 0);
      await tester.sendKeyEvent(LogicalKeyboardKey.delete);
      await tester.pump();
      expect(controller.text, isEmpty);
    },
    variant: const TargetPlatformVariant({TargetPlatform.linux}),
  );
}
