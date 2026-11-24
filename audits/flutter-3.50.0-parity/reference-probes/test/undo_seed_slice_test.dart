import 'package:flutter/widgets.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  void scenario(String name, int delay, int deletes, List<String> expected) {
    testWidgets(name, (t) async {
      final text = TextEditingController(text: 'abc');
      final focus = FocusNode();
      final history = UndoHistoryController();
      await t.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: DefaultTextEditingShortcuts(
            child: EditableText(
              controller: text,
              focusNode: focus,
              undoController: history,
              style: const TextStyle(),
              cursorColor: const Color(0xff000000),
              backgroundCursorColor: const Color(0xff000000),
            ),
          ),
        ),
      );
      try {
        focus.requestFocus();
        await t.pump();
        text.selection = const TextSelection.collapsed(offset: 3);
        await t.pump(Duration(milliseconds: delay));
        for (var i = 0; i < deletes; i++) {
          await t.sendKeyEvent(LogicalKeyboardKey.backspace);
          await t.pump(const Duration(milliseconds: 16));
        }
        await t.pump(const Duration(milliseconds: 600));
        String snapshot() =>
            '${text.text} caret=${text.selection.extentOffset} ${history.value.canUndo}/${history.value.canRedo}';
        final states = [snapshot()];
        final texts = [text.text];
        history.undo();
        await t.pump();
        states.add(snapshot());
        texts.add(text.text);
        history.redo();
        await t.pump();
        states.add(snapshot());
        texts.add(text.text);
        print(states);
        expect(texts, expected);
      } finally {
        await t.pumpWidget(const SizedBox.shrink());
        history.dispose();
        focus.dispose();
        text.dispose();
      }
    });
  }

  scenario('unseeded single edit', 0, 1, ['ab', 'ab', 'ab']);
  scenario('seeded single edit', 600, 1, ['ab', 'abc', 'ab']);
  scenario('seeded rapid pair', 600, 2, ['a', 'abc', 'a']);
}
