import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('floating caret commits only collapsed selection after settling', (tester) async {
    final controller = TextEditingController(text: 'ABCDEFGHIJ');
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
          cursorColor: const Color(0xFF111FFB),
          backgroundCursorColor: const Color(0xFF333333),
        ),
      ),
    );
    focus.requestFocus();
    await tester.pump();
    final attach = tester.testTextInput.log.lastWhere((call) => call.method == 'TextInput.setClient');
    final client = (attach.arguments as List<dynamic>).first as int;

    Future<void> cursor(String phase, num x, num y, [int? sender]) async {
      await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        'flutter/textinput',
        const JSONMethodCodec().encodeMethodCall(
          MethodCall('TextInputClient.updateFloatingCursor', <dynamic>[
            sender ?? client,
            'FloatingCursorDragState.$phase',
            <String, num>{'X': x, 'Y': y},
          ]),
        ),
        (_) {},
      );
    }

    controller.selection = const TextSelection.collapsed(offset: 0);
    await tester.pump();
    await cursor('start', 100, 100);
    await cursor('update', 56, 0);
    expect(controller.selection.baseOffset, 0);
    await cursor('end', 0, 0);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 60));
    expect(controller.selection.baseOffset, 0);
    await tester.pump(const Duration(milliseconds: 70));
    expect(controller.selection.baseOffset, 4);
    expect(controller.selection.extentOffset, 4);

    controller.selection = const TextSelection.collapsed(offset: 2);
    await tester.pump();
    await cursor('start', 0, 0, client + 1);
    await cursor('start', 0, 0);
    await cursor('update', 42, 0);
    controller.selection = const TextSelection(baseOffset: 2, extentOffset: 5);
    await cursor('end', 0, 0);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 130));
    expect(controller.selection.baseOffset, 2);
    expect(controller.selection.extentOffset, 5);
    expect(controller.text, 'ABCDEFGHIJ');
  });
}
