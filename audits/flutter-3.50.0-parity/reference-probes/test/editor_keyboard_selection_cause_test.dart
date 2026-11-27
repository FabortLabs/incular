import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('physical arrow keys report a keyboard selection cause', (WidgetTester tester) async {
    final TextEditingController controller = TextEditingController(text: 'abcdef');
    final FocusNode focus = FocusNode();
    final List<(TextSelection, SelectionChangedCause?)> changes = [];
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    controller.selection = const TextSelection.collapsed(offset: 2);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: DefaultTextEditingShortcuts(
          child: EditableText(
            controller: controller,
            focusNode: focus,
            autofocus: true,
            style: const TextStyle(fontSize: 14),
            cursorColor: const Color(0xffffffff),
            backgroundCursorColor: const Color(0xff333333),
            onSelectionChanged: (selection, cause) => changes.add((selection, cause)),
          ),
        ),
      ),
    );
    await tester.pump();
    changes.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pump();
    expect(controller.selection, const TextSelection.collapsed(offset: 3));
    expect(changes, [(const TextSelection.collapsed(offset: 3), SelectionChangedCause.keyboard)]);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pump();
    expect(controller.selection.baseOffset, 3);
    expect(controller.selection.extentOffset, 4);
    expect(changes, hasLength(2));
    expect(changes.last.$2, SelectionChangedCause.keyboard);

    controller.selection = const TextSelection.collapsed(offset: 1);
    await tester.pump();
    expect(changes, hasLength(2), reason: 'controller setters are not user selection changes');
  });
}
