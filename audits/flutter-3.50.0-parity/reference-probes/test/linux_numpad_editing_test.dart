import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Physical NumLock controls keypad editing on a focused Linux paragraph', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController(text: 'abc def');
    final FocusNode focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(WidgetsApp(
      color: const Color(0xff000000),
      builder: (BuildContext context, Widget? child) => Center(
        child: SizedBox(
          width: 300,
          child: EditableText(
            controller: controller,
            focusNode: focus,
            style: const TextStyle(fontSize: 10, color: Color(0xff000000)),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xff000000),
          ),
        ),
      ),
    ));
    focus.requestFocus();
    await tester.pump();
    controller.selection = const TextSelection.collapsed(offset: 3);

    await tester.sendKeyEvent(LogicalKeyboardKey.numLock);
    expect(HardwareKeyboard.instance.lockModesEnabled, contains(KeyboardLockMode.numLock));
    await tester.sendKeyEvent(LogicalKeyboardKey.numpad6);
    expect(controller.selection, const TextSelection.collapsed(offset: 3));
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.numpad6);
    expect(controller.selection, const TextSelection(baseOffset: 3, extentOffset: 4));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);

    await tester.sendKeyEvent(LogicalKeyboardKey.numLock);
    expect(HardwareKeyboard.instance.lockModesEnabled, isNot(contains(KeyboardLockMode.numLock)));
    controller.selection = const TextSelection.collapsed(offset: 3);
    await tester.sendKeyEvent(LogicalKeyboardKey.numpad6);
    expect(controller.selection, const TextSelection.collapsed(offset: 4));

    controller.selection = const TextSelection.collapsed(offset: 0);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.numpad6);
    expect(controller.selection, const TextSelection.collapsed(offset: 3));
    controller.selection = const TextSelection.collapsed(offset: 0);
    await tester.sendKeyEvent(LogicalKeyboardKey.numpadDecimal);
    expect(controller.text, ' def');
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  }, variant: TargetPlatformVariant.only(TargetPlatform.linux));
}
