import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('the mounted editor handles active, closed and stale host focus messages', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'test');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    int submissions = 0;
    int completions = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xFF000000),
          backgroundCursorColor: const Color(0xFF333333),
          onEditingComplete: () => completions += 1,
          onSubmitted: (_) => submissions += 1,
        ),
      ),
    );
    int activeId() {
      final attach = tester.testTextInput.log.lastWhere(
        (MethodCall call) => call.method == 'TextInput.setClient',
      );
      return (attach.arguments as List<dynamic>)[0] as int;
    }

    Future<Object?> notify(String method, int id) async {
      const codec = JSONMethodCodec();
      ByteData? reply;
      await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        'flutter/textinput',
        codec.encodeMethodCall(MethodCall('TextInputClient.$method', <dynamic>[id])),
        (bytes) => reply = bytes,
      );
      return reply == null ? null : codec.decodeEnvelope(reply!);
    }

    focus.requestFocus();
    await tester.pump();
    final first = activeId();
    expect(focus.hasPrimaryFocus, isTrue);
    tester.testTextInput.log.clear();

    expect(await notify('onFocusReceived', first), isFalse);
    await tester.pump();
    expect(focus.hasPrimaryFocus, isTrue);
    expect(
      tester.testTextInput.log.where(
        (call) => const {'TextInput.setClient', 'TextInput.clearClient', 'TextInput.show', 'TextInput.hide'}.contains(call.method),
      ),
      isEmpty,
    );
    tester.testTextInput.log.clear();

    await notify('onConnectionClosed', first);
    await tester.pump();
    expect(focus.hasFocus, isFalse);
    expect(controller.text, 'test');
    expect(completions, 0);
    expect(submissions, 0);
    expect(tester.testTextInput.log, isEmpty);

    expect(await notify('onFocusReceived', first), isTrue);
    await tester.pump();
    expect(focus.hasPrimaryFocus, isTrue);
    expect(activeId(), isNot(first));
    expect(await notify('onFocusReceived', first), isFalse);
    expect(focus.hasPrimaryFocus, isTrue);
  });
}
