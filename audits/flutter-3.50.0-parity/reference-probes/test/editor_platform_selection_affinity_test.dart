import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('platform selection keeps the local visual edge unless its origin changes', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController(text: 'first line');
    final FocusNode focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    controller.selection = const TextSelection.collapsed(offset: 1);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          autofocus: true,
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xffffffff),
          backgroundCursorColor: const Color(0xff333333),
        ),
      ),
    );
    await tester.pump();
    final MethodCall attach = tester.testTextInput.log.lastWhere(
      (MethodCall call) => call.method == 'TextInput.setClient',
    );
    final int client = (attach.arguments as List<dynamic>).first as int;
    Future<void> update(
      String text,
      int base,
      int extent,
      String affinity, {
      bool directional = false,
    }) async {
      await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        'flutter/textinput',
        const JSONMethodCodec().encodeMethodCall(
          MethodCall('TextInputClient.updateEditingState', <dynamic>[
            client,
            <String, dynamic>{
              'text': text,
              'selectionBase': base,
              'selectionExtent': extent,
              'selectionAffinity': affinity,
              'selectionIsDirectional': directional,
              'composingBase': -1,
              'composingExtent': -1,
            },
          ]),
        ),
        (_) {},
      );
      await tester.pump();
    }

    await update('first line', 2, 2, 'TextAffinity.upstream');
    expect(controller.selection.affinity, TextAffinity.upstream);
    await update('first line', 2, 2, 'TextAffinity.downstream');
    expect(controller.selection.affinity, TextAffinity.upstream);
    await update('First line', 2, 2, 'TextAffinity.downstream');
    expect(controller.selection.affinity, TextAffinity.downstream);
    await update('First line', 3, 3, 'TextAffinity.upstream');
    expect(controller.selection.affinity, TextAffinity.upstream);
    await update('First line', 4, 4, 'unknown');
    expect(controller.selection.affinity, TextAffinity.downstream);

    await update('a😀z', 3, 1, 'TextAffinity.upstream', directional: true);
    expect(controller.selection.baseOffset, 3);
    expect(controller.selection.extentOffset, 1);
    expect(controller.selection.isDirectional, isTrue);
    expect(controller.selection.affinity, TextAffinity.upstream);
  });
}
