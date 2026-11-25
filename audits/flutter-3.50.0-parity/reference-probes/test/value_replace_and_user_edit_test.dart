import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('paste collapses the caret but value replacement remaps it', (
    tester,
  ) async {
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
      return switch (call.method) {
        'Clipboard.getData' => <String, Object>{'text': 'X'},
        'Clipboard.hasStrings' => <String, Object>{'value': true},
        _ => null,
      };
    });
    addTearDown(
      () => messenger.setMockMethodCallHandler(SystemChannels.platform, null),
    );
    final selection = TextSelection(baseOffset: 4, extentOffset: 1);
    final source = TextEditingValue(text: 'ab😀c', selection: selection);
    final controller = TextEditingController.fromValue(source);
    final focus = FocusNode();
    final key = GlobalKey<EditableTextState>();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          key: key,
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
          cursorColor: const Color(0xff000000),
          backgroundCursorColor: const Color(0xff808080),
        ),
      ),
    );
    focus.requestFocus();
    await tester.pump();
    controller.selection = selection;
    final replaced = source.replaced(selection, 'X');
    expect(replaced.text, 'aXc');
    expect(
      replaced.selection,
      const TextSelection(baseOffset: 2, extentOffset: 1),
    );
    await key.currentState!.pasteText(SelectionChangedCause.keyboard);
    expect(controller.text, 'aXc');
    expect(controller.selection, const TextSelection.collapsed(offset: 2));
  });

  testWidgets('deleting a selected range collapses at its original start', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'hello world');
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
            style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xff808080),
          ),
        ),
      ),
    );
    focus.requestFocus();
    await tester.pump();
    controller.selection = const TextSelection(baseOffset: 2, extentOffset: 7);
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    expect(controller.text, 'heorld');
    expect(controller.selection, const TextSelection.collapsed(offset: 2));
  });
}
