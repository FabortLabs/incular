import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('private input commands reach only the active editor unchanged', (
    tester,
  ) async {
    final a = TextEditingController(text: 'one');
    final b = TextEditingController(text: 'two');
    final first = FocusNode();
    final second = FocusNode();
    addTearDown(a.dispose);
    addTearDown(b.dispose);
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    final calls = <(String, String, Map<String, dynamic>)>[];
    Widget editor(
      String name,
      TextEditingController controller,
      FocusNode focus,
    ) {
      return SizedBox(
        height: 40,
        child: EditableText(
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xFF000000),
          backgroundCursorColor: const Color(0xFF333333),
          onAppPrivateCommand: (action, data) =>
              calls.add((name, action, data)),
        ),
      );
    }

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Column(
          children: [editor('first', a, first), editor('second', b, second)],
        ),
      ),
    );
    int activeId() {
      final MethodCall attach = tester.testTextInput.log.lastWhere(
        (MethodCall call) => call.method == 'TextInput.setClient',
      );
      return (attach.arguments as List<dynamic>)[0] as int;
    }

    first.requestFocus();
    await tester.pump();
    final int stale = activeId();
    second.requestFocus();
    await tester.pump();
    final int active = activeId();
    expect(stale, isNot(active));
    tester.testTextInput.log.clear();

    Future<void> send(int client, Object? payload) async {
      await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        'flutter/textinput',
        const JSONMethodCodec().encodeMethodCall(
          MethodCall('TextInputClient.performPrivateCommand', <dynamic>[
            client,
            payload,
          ]),
        ),
        (_) {},
      );
    }

    await send(stale, <String, dynamic>{
      'action': 'old',
      'data': <String, dynamic>{'bad': 2},
    });
    expect(calls, isEmpty);
    await send(active, <String, dynamic>{'action': 'example', 'data': null});
    await send(active, <String, dynamic>{
      'action': 'example',
      'data': <String, dynamic>{
        'items': <dynamic>[0.5, 'abc'],
      },
    });
    expect(calls.length, 2);
    expect(calls[0].$1, 'second');
    expect(calls[0].$2, 'example');
    expect(calls[0].$3, isEmpty);
    expect(calls[1].$1, 'second');
    expect(calls[1].$2, 'example');
    expect(calls[1].$3, <String, dynamic>{
      'items': <dynamic>[0.5, 'abc'],
    });
    expect(a.text, 'one');
    expect(b.text, 'two');
    expect(second.hasPrimaryFocus, isTrue);
    expect(tester.testTextInput.log, isEmpty);
  });
}
