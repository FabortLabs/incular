import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'an AppKit character selector uses the focused editor action override',
    (tester) async {
      final text = TextEditingController(text: 'Some text');
      final focus = FocusNode();
      addTearDown(text.dispose);
      addTearDown(focus.dispose);
      text.selection = const TextSelection.collapsed(offset: 0);
      var calls = 0;
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xff000000),
          builder: (_, _) => Actions(
            actions: {
              ExtendSelectionByCharacterIntent:
                  CallbackAction<ExtendSelectionByCharacterIntent>(
                    onInvoke: (intent) {
                      expect(intent.forward, isTrue);
                      expect(intent.collapseSelection, isTrue);
                      calls++;
                      return null;
                    },
                  ),
            },
            child: EditableText(
              autofocus: true,
              controller: text,
              focusNode: focus,
              style: const TextStyle(),
              cursorColor: const Color(0xff000000),
              backgroundCursorColor: const Color(0xff000000),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(focus.hasPrimaryFocus, isTrue);
      await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        'flutter/textinput',
        const JSONMethodCodec().encodeMethodCall(
          const MethodCall('TextInputClient.performSelectors', [
            -1,
            ['moveRight:'],
          ]),
        ),
        (_) {},
      );
      expect(calls, 1);
      expect(text.selection, const TextSelection.collapsed(offset: 0));
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
