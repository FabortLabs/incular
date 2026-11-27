import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('host toolbar request shows once without editing the selection', (tester) async {
    final controller = TextEditingController(text: 'hello world');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    const toolbar = ValueKey<String>('platform-toolbar');
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Overlay(
          initialEntries: [
            OverlayEntry(
              builder: (_) => EditableText(
                controller: controller,
                focusNode: focus,
                style: const TextStyle(fontSize: 14),
                cursorColor: const Color(0xFF000000),
                backgroundCursorColor: const Color(0xFF333333),
                contextMenuBuilder: (_, _) => const SizedBox(key: toolbar, width: 32, height: 16),
              ),
            ),
          ],
        ),
      ),
    );
    focus.requestFocus();
    await tester.pump();
    const selection = TextSelection(baseOffset: 0, extentOffset: 5);
    controller.selection = selection;
    await tester.pump();
    final attach = tester.testTextInput.log.lastWhere((call) => call.method == 'TextInput.setClient');
    final client = (attach.arguments as List<dynamic>).first as int;

    Future<void> show(int id) async {
      await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        'flutter/textinput',
        const JSONMethodCodec().encodeMethodCall(
          MethodCall('TextInputClient.showToolbar', <dynamic>[id, 0, 1]),
        ),
        (_) {},
      );
      await tester.pump();
    }

    await show(client + 1);
    expect(find.byKey(toolbar), findsNothing);
    await show(client);
    expect(find.byKey(toolbar), findsOneWidget);
    expect(controller.selection, selection);
    await show(client);
    expect(find.byKey(toolbar), findsOneWidget);
    expect(controller.selection, selection);
  });
}
