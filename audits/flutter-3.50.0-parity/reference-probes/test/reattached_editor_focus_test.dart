import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('an editor reattached to the same FocusNode keeps its selection', (tester) async {
    final node = FocusNode();
    final controller = TextEditingController(text: 'ABCDE');
    addTearDown(node.dispose);
    addTearDown(controller.dispose);

    Widget view(bool wrapped) {
      final editor = EditableText(
        controller: controller,
        focusNode: node,
        style: const TextStyle(fontSize: 14),
        cursorColor: const Color(0xff0044dd),
        backgroundCursorColor: const Color(0xff555555),
      );
      return Directionality(
        textDirection: TextDirection.ltr,
        child: Center(child: SizedBox(
          width: 200,
          child: wrapped ? Padding(padding: const EdgeInsets.all(5), child: editor) : editor,
        )),
      );
    }

    await tester.pumpWidget(view(false));
    node.requestFocus();
    await tester.pump();
    expect(node.hasPrimaryFocus, isTrue);
    const selection = TextSelection(baseOffset: 1, extentOffset: 3);
    controller.selection = selection;

    await tester.pumpWidget(view(true));
    expect(node.hasPrimaryFocus, isTrue);
    expect(controller.text, 'ABCDE');
    expect(controller.selection, selection);
  });
}
