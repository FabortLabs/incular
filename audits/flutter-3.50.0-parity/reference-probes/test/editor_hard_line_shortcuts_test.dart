import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'home and end keep the caret on the line before a hard break',
    (WidgetTester tester) async {
      final controller = TextEditingController();
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
              style: const TextStyle(fontSize: 14),
              cursorColor: const Color(0xFF000000),
              backgroundCursorColor: const Color(0xFFFFFFFF),
              maxLines: null,
            ),
          ),
        ),
      );
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);
      final EditableTextState field = tester.state<EditableTextState>(
        find.byType(EditableText),
      );
      double caretY() => field.renderEditable
          .getLocalRectForCaret(controller.selection.extent)
          .top;
      final String platform = defaultTargetPlatform.name;
      Future<void> key(LogicalKeyboardKey key, {bool shift = false}) async {
        if (shift) {
          await tester.sendKeyDownEvent(
            LogicalKeyboardKey.shiftLeft,
            platform: platform,
          );
        }
        await tester.sendKeyEvent(key, platform: platform);
        if (shift) {
          await tester.sendKeyUpEvent(
            LogicalKeyboardKey.shiftLeft,
            platform: platform,
          );
        }
        await tester.pump();
      }

      for (final String text in [
        'abcd\nmiddle\nlast',
        'aébc\r\nmiddle\nlast',
      ]) {
        final int logicalEnd = text.indexOf('\n');
        controller.text = text;
        controller.selection = const TextSelection.collapsed(offset: 4);
        await tester.pump();
        await key(LogicalKeyboardKey.home);
        expect(
          controller.selection,
          const TextSelection.collapsed(offset: 0),
          reason: '$platform: $text',
        );

        final double firstLineY = caretY();
        controller.selection = const TextSelection.collapsed(offset: 4);
        await tester.pump();
        await key(LogicalKeyboardKey.end);
        expect(
          controller.selection.baseOffset,
          logicalEnd,
          reason: '$platform: $text',
        );
        expect(
          controller.selection.extentOffset,
          logicalEnd,
          reason: '$platform: $text',
        );

        expect(
          caretY(),
          firstLineY,
          reason: "End keeps the caret on the first drawn line",
        );
        controller.selection = const TextSelection.collapsed(offset: 4);
        await tester.pump();
        await key(LogicalKeyboardKey.home, shift: true);
        expect(controller.selection.baseOffset, 4, reason: '$platform: $text');
        expect(
          controller.selection.extentOffset,
          0,
          reason: '$platform: $text',
        );

        controller.selection = const TextSelection.collapsed(offset: 1);
        await tester.pump();
        await key(LogicalKeyboardKey.end, shift: true);
        expect(controller.selection.baseOffset, 1, reason: '$platform: $text');
        expect(
          controller.selection.extentOffset,
          logicalEnd,
          reason: '$platform: $text',
        );
      }
    },
    variant: const TargetPlatformVariant({
      TargetPlatform.linux,
      TargetPlatform.windows,
    }),
  );
}
