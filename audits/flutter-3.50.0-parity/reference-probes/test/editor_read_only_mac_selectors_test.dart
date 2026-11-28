import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a read-only macOS editor receives native selection and focus selectors',
    (tester) async {
      final controller = TextEditingController(
        text: 'Lorem ipsum dolor sit amet',
      );
      final otherController = TextEditingController();
      final focus = FocusNode();
      final other = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(otherController.dispose);
      addTearDown(focus.dispose);
      addTearDown(other.dispose);
      EditableText field(
        TextEditingController text,
        FocusNode node,
        bool readOnly,
      ) => EditableText(
        controller: text,
        focusNode: node,
        readOnly: readOnly,
        autofocus: readOnly,
        style: const TextStyle(),
        cursorColor: const Color(0xff000000),
        backgroundCursorColor: const Color(0xff000000),
      );
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xff000000),
          builder: (_, _) => Directionality(
            textDirection: TextDirection.ltr,
            child: Column(
              children: [
                field(controller, focus, true),
                field(otherController, other, false),
              ],
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(focus.hasPrimaryFocus, isTrue);
      expect(tester.testTextInput.setClientArgs!['readOnly'], isTrue);
      // testTextInput uses the framework's debug "current client" sentinel.
      Future<void> selectors(List<String> values) async {
        await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
          'flutter/textinput',
          const JSONMethodCodec().encodeMethodCall(
            MethodCall('TextInputClient.performSelectors', [-1, values]),
          ),
          (_) {},
        );
        await tester.pump();
      }

      controller.selection = const TextSelection.collapsed(offset: 0);
      await selectors(['moveRight:', 'deleteBackward:']);
      expect(controller.selection, const TextSelection.collapsed(offset: 1));
      expect(controller.text, 'Lorem ipsum dolor sit amet');
      await selectors(['insertTab:']);
      expect(other.hasPrimaryFocus, isTrue);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
