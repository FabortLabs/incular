import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('mounted editor applies selectors in order across a surrogate pair', (tester) async {
    final controller = TextEditingController(text: 'AB😀C');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xFF000000),
          backgroundCursorColor: const Color(0xFF333333),
        ),
      ),
    );
    focus.requestFocus();
    await tester.pump();
    controller.selection = TextSelection.collapsed(offset: controller.text.length);
    final attach = tester.testTextInput.log.lastWhere((call) => call.method == 'TextInput.setClient');
    final client = (attach.arguments as List<dynamic>).first as int;

    Future<void> selectors(int id, List<String> names) async {
      await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        'flutter/textinput',
        const JSONMethodCodec().encodeMethodCall(
          MethodCall('TextInputClient.performSelectors', <dynamic>[id, names]),
        ),
        (_) {},
      );
    }

    await selectors(client, ['moveLeft:', 'unbound:', 'deleteBackward:', 'moveRightAndModifySelection:']);
    expect(controller.text, 'ABC');
    expect(controller.selection.baseOffset, 2);
    expect(controller.selection.extentOffset, 3);
    await selectors(client, ['deleteBackward:']);
    expect(controller.text, 'AB');
    expect(controller.selection.baseOffset, 2);
    expect(controller.selection.extentOffset, 2);
    await selectors(client + 1, ['deleteBackward:']);
    expect(controller.text, 'AB');
  });
}
