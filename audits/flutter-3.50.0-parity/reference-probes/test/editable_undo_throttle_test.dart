import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Future<(TextEditingController, UndoHistoryController)> mountEditor(
  WidgetTester tester,
) async {
  final text = TextEditingController.fromValue(
    const TextEditingValue(
      text: 'abc',
      selection: TextSelection.collapsed(offset: 3),
    ),
  );
  final focus = FocusNode();
  final history = UndoHistoryController();
  addTearDown(text.dispose);
  addTearDown(focus.dispose);
  addTearDown(history.dispose);
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: DefaultTextEditingShortcuts(
        child: EditableText(
          controller: text,
          focusNode: focus,
          undoController: history,
          style: const TextStyle(),
          cursorColor: const Color(0xFF000000),
          backgroundCursorColor: const Color(0xFF000000),
        ),
      ),
    ),
  );
  focus.requestFocus();
  await tester.pump();
  return (text, history);
}

void main() {
  for (final (baselineDelay, deletes, undoValue, undoPossible) in [
    (0, 1, 'ab', false),
    (600, 1, 'abc', true),
    (600, 2, 'abc', true),
  ]) {
    testWidgets('undo baseline $baselineDelay ms, $deletes deletions', (
      tester,
    ) async {
      final (text, history) = await mountEditor(tester);
      await tester.pump(Duration(milliseconds: baselineDelay));
      for (var i = 0; i < deletes; i++) {
        await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
        await tester.pump(const Duration(milliseconds: 16));
      }
      final result = deletes == 1 ? 'ab' : 'a';
      expect(text.text, result);
      expect(
        history.value.canUndo,
        false,
        reason: 'the group is still pending',
      );
      await tester.pump(const Duration(milliseconds: 600));
      expect(history.value.canUndo, undoPossible);
      history.undo();
      await tester.pump();
      expect(text.text, undoValue);
      expect(text.selection.extentOffset, undoValue.length);
      expect(history.value.canRedo, undoPossible);
      history.redo();
      await tester.pump();
      expect(text.text, result);
    });
  }

  testWidgets('keyboard undo cancels a pending group without creating redo', (
    tester,
  ) async {
    final (text, history) = await mountEditor(tester);
    await tester.pump(const Duration(milliseconds: 600));
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pump(const Duration(milliseconds: 16));
    expect(text.text, 'ab');
    final historyState = tester.state<UndoHistoryState<TextEditingValue>>(
      find.byWidgetPredicate(
        (widget) => widget is UndoHistory<TextEditingValue>,
      ),
    );
    historyState.undo();
    await tester.pump();
    expect(text.text, 'abc');
    expect(text.selection.extentOffset, 3);
    expect(history.value.canUndo, false);
    expect(history.value.canRedo, false);
    await tester.pump(const Duration(milliseconds: 600));
    expect(history.value.canUndo, false);
  });
}
