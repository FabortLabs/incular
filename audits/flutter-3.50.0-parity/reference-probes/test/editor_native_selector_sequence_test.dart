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

  testWidgets('native Tab and Backtab retire the previous focused attachment', (tester) async {
    final first = FocusNode();
    final second = FocusNode();
    final a = TextEditingController(text: 'first');
    final b = TextEditingController(text: 'second');
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    addTearDown(a.dispose);
    addTearDown(b.dispose);

    Widget editor(FocusNode focus, TextEditingController controller) => SizedBox(
      height: 40,
      child: EditableText(
        controller: controller,
        focusNode: focus,
        style: const TextStyle(fontSize: 14),
        cursorColor: const Color(0xFF000000),
        backgroundCursorColor: const Color(0xFF333333),
      ),
    );

    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFF000000),
        builder: (_, _) => Column(children: [editor(first, a), editor(second, b)]),
      ),
    );
    int activeClient() {
      final attach = tester.testTextInput.log.lastWhere((call) => call.method == 'TextInput.setClient');
      return (attach.arguments as List<dynamic>).first as int;
    }
    Future<void> selector(int id, String name) async {
      await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        'flutter/textinput',
        const JSONMethodCodec().encodeMethodCall(
          MethodCall('TextInputClient.performSelectors', <dynamic>[id, <String>[name]]),
        ),
        (_) {},
      );
      await tester.pump();
    }

    first.requestFocus();
    await tester.pump();
    final original = activeClient();
    expect(first.hasPrimaryFocus, isTrue);

    await selector(original, 'insertTab:');
    expect(second.hasPrimaryFocus, isTrue);
    final next = activeClient();
    expect(next, isNot(original));

    await selector(original, 'insertBacktab:');
    expect(second.hasPrimaryFocus, isTrue);
    expect(activeClient(), next);

    await selector(next, 'insertBacktab:');
    final returned = activeClient();
    expect(first.hasPrimaryFocus, isTrue);
    expect(returned, isNot(original));
    expect(returned, isNot(next));
    expect(a.text, 'first');
    expect(b.text, 'second');
  });
}
