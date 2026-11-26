import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('separated item lookup preserves a parked focused editor', (
    tester,
  ) async {
    final editor = GlobalKey<EditableTextState>();
    final focus = FocusNode();
    final text = TextEditingController(text: 'draft');
    final scroll = ScrollController();
    addTearDown(focus.dispose);
    addTearDown(text.dispose);
    addTearDown(scroll.dispose);
    Widget build(int slot) => MediaQuery(
      data: const MediaQueryData(),
      child: Directionality(
        textDirection: TextDirection.ltr,
        child: ListView.separated(
          controller: scroll,
          itemCount: 100,
          itemBuilder: (_, index) => index == slot
              ? EditableText(
                  key: editor,
                  controller: text,
                  focusNode: focus,
                  autofocus: true,
                  style: const TextStyle(fontSize: 14),
                  cursorColor: const Color(0xff000000),
                  backgroundCursorColor: const Color(0xff000000),
                )
              : const SizedBox(height: 40),
          separatorBuilder: (_, _) => const SizedBox(height: 2),
          findItemIndexCallback: (key) => key == editor ? slot : null,
        ),
      ),
    );

    await tester.pumpWidget(build(0));
    await tester.pump();
    final state = editor.currentState!;
    expect(focus.hasPrimaryFocus, isTrue);

    await tester.pumpWidget(build(90));
    expect(editor.currentState, same(state));
    expect(focus.hasPrimaryFocus, isTrue);
    expect(scroll.offset, 0);
    expect(text.text, 'draft');

    tester.testTextInput.closeConnection();
    await tester.pumpAndSettle();
    expect(editor.currentState, isNull);
    expect(focus.hasFocus, isFalse);
  });
}
