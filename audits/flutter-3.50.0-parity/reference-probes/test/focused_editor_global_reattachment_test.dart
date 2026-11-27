import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('A focused editor moving into LayoutBuilder keeps the editing state', (tester) async {
    final key = GlobalKey<EditableTextState>();
    final node = FocusNode();
    final controller = TextEditingController(text: 'abcd');
    addTearDown(node.dispose);
    addTearDown(controller.dispose);
    final editor = EditableText(
      key: key,
      focusNode: node,
      controller: controller,
      style: const TextStyle(fontSize: 16),
      cursorColor: const Color(0xff000000),
      backgroundCursorColor: const Color(0xff000000),
    );
    Widget host(Widget child) =>
        Directionality(textDirection: TextDirection.ltr, child: Column(children: [child]));

    await tester.pumpWidget(host(editor));
    node.requestFocus();
    await tester.pump();
    final original = key.currentState;
    const selection = TextSelection(baseOffset: 1, extentOffset: 3);
    const composing = TextRange(start: 1, end: 3);
    controller.value = controller.value.copyWith(selection: selection, composing: composing);
    await tester.pumpWidget(host(LayoutBuilder(builder: (_, _) => editor)));

    expect(key.currentState, same(original));
    expect(node.hasPrimaryFocus, isTrue);
    expect(controller.selection, selection);
    expect(controller.value.composing, composing);
  });
}
