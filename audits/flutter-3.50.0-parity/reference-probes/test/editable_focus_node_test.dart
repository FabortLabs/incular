import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget editor(
  TextEditingController controller,
  FocusNode focus, {
  bool autofocus = false,
}) => Directionality(
  textDirection: TextDirection.ltr,
  child: EditableText(
    controller: controller,
    focusNode: focus,
    autofocus: autofocus,
    style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
    cursorColor: const Color(0xff000000),
    backgroundCursorColor: const Color(0xff808080),
  ),
);

void main() {
  testWidgets('editable binds replacement node to the same editing value', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'ab');
    final first = FocusNode();
    final next = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(first.dispose);
    addTearDown(next.dispose);
    await tester.pumpWidget(editor(controller, first));
    expect(first.hasPrimaryFocus, false);
    first.requestFocus();
    await tester.pump();
    tester.testTextInput.enterText('ab!');
    await tester.pump();
    expect(controller.text, 'ab!');
    expect(first.hasPrimaryFocus, true);

    next.requestFocus();
    await tester.pumpWidget(editor(controller, next));
    await tester.pump();
    expect((first.hasPrimaryFocus, next.hasPrimaryFocus), (false, true));
    first.requestFocus();
    await tester.pump();
    expect((first.hasPrimaryFocus, next.hasPrimaryFocus), (false, true));
    tester.testTextInput.enterText('ab!?');
    await tester.pump();
    expect(controller.text, 'ab!?');
    expect(controller.selection.extentOffset, 4);
  });

  testWidgets('autofocus only grants an eligible editable node', (tester) async {
    for (final eligible in [false, true]) {
      final controller = TextEditingController();
      final focus = FocusNode(canRequestFocus: eligible);
      addTearDown(controller.dispose);
      addTearDown(focus.dispose);
      await tester.pumpWidget(editor(controller, focus, autofocus: true));
      await tester.pump();
      expect(focus.hasPrimaryFocus, eligible);
      await tester.pumpWidget(const SizedBox.shrink());
    }
  });
}
