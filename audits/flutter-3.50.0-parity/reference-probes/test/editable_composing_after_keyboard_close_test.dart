import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('clearing composition does not make an unset selection valid', () {
    final controller = TextEditingController(text: 'draft');
    addTearDown(controller.dispose);
    expect(controller.selection.isValid, isFalse);
    controller.value = controller.value.copyWith(composing: const TextRange(start: 1, end: 4));
    expect(controller.selection.isValid, isFalse);
    controller.clearComposing();
    expect(controller.value.composing, TextRange.empty);
    expect(controller.text, 'draft');
    expect(controller.selection.isValid, isFalse);
  });

  testWidgets('keyboard close and focus transfer both clear only the old composing range', (
    WidgetTester tester,
  ) async {
    final controller = TextEditingController();
    final first = FocusNode();
    final second = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    const firstKey = ValueKey('first');
    const secondKey = ValueKey('second');
    Widget editor(Key key, FocusNode focus, TextInputType keyboard) => SizedBox(
      height: 40,
      child: EditableText(
        key: key,
        controller: controller,
        focusNode: focus,
        keyboardType: keyboard,
        style: const TextStyle(fontSize: 14),
        cursorColor: const Color(0xFF000000),
        backgroundCursorColor: const Color(0xFF333333),
      ),
    );
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Column(
          children: [
            editor(firstKey, first, TextInputType.text),
            editor(secondKey, second, TextInputType.phone),
          ],
        ),
      ),
    );
    const composing = TextEditingValue(
      text: 'draft',
      selection: TextSelection.collapsed(offset: 3),
      composing: TextRange(start: 1, end: 4),
    );
    final firstEditor = find.byKey(firstKey);
    final secondEditor = find.byKey(secondKey);
    await tester.tap(firstEditor);
    await tester.showKeyboard(firstEditor);
    tester.testTextInput.updateEditingValue(composing);
    await tester.pump();
    expect(controller.value, composing);

    tester.testTextInput.log.clear();
    tester.testTextInput.closeConnection();
    await tester.pump();
    expect(first.hasFocus, isFalse);
    expect(controller.value, composing.copyWith(composing: TextRange.empty));
    expect(tester.testTextInput.log, isEmpty);

    await tester.tap(secondEditor);
    await tester.showKeyboard(secondEditor);
    expect(second.hasPrimaryFocus, isTrue);
    expect(tester.testTextInput.editingState?['composingBase'], -1);
    expect(tester.testTextInput.editingState?['text'], 'draft');

    tester.testTextInput.updateEditingValue(composing);
    await tester.pump();
    expect(controller.value, composing);
    first.requestFocus();
    await tester.pump();
    expect(first.hasPrimaryFocus, isTrue);
    expect(controller.value, composing.copyWith(composing: TextRange.empty));
    expect(tester.testTextInput.editingState?['composingBase'], -1);
  });
}
