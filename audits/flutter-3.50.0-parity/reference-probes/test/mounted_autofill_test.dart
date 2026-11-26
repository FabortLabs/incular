import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget editor(
  TextEditingController controller,
  FocusNode node,
  List<String>? hints, {
  bool readOnly = false,
  ValueChanged<String>? onChanged,
}) {
  return EditableText(
    controller: controller,
    focusNode: node,
    style: const TextStyle(),
    cursorColor: const Color(0xff000000),
    backgroundCursorColor: const Color(0xff777777),
    autofillHints: hints,
    readOnly: readOnly,
    onChanged: onChanged,
  );
}

Map<Object?, Object?> lastConfiguration(WidgetTester tester) =>
    (tester.testTextInput.log
                .lastWhere((call) => call.method == 'TextInput.setClient')
                .arguments
            as List<Object?>)[1]!
        as Map<Object?, Object?>;

String fieldTag(Map<Object?, Object?> configuration, String hint) {
  final fields = configuration['fields']! as List<Object?>;
  for (final raw in fields) {
    final field = raw! as Map<Object?, Object?>;
    final autofill = field['autofill']! as Map<Object?, Object?>;
    if ((autofill['hints']! as List<Object?>).first == hint) {
      return autofill['uniqueIdentifier']! as String;
    }
  }
  throw StateError('Unknown field in fixture: $hint');
}

Future<void> fill(
  WidgetTester tester,
  Map<String, TextEditingValue> values,
) async {
  final message = SystemChannels.textInput.codec.encodeMethodCall(
    MethodCall('TextInputClient.updateEditingStateWithTag', <Object?>[
      0,
      values.map((tag, value) => MapEntry(tag, value.toJSON())),
    ]),
  );
  await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
    SystemChannels.textInput.name,
    message,
    (_) {},
  );
  await tester.pump();
}

void main() {
  testWidgets(
    'tagged autofill updates a nonfocused editor only in its mounted scope',
    (tester) async {
      final first = TextEditingController(text: 'first');
      final second = TextEditingController(text: 'second');
      final nested = TextEditingController(text: 'nested');
      final readonly = TextEditingController(text: 'read only');
      final firstFocus = FocusNode();
      final secondFocus = FocusNode();
      final nestedFocus = FocusNode();
      final readonlyFocus = FocusNode();
      addTearDown(() {
        for (final node in [
          firstFocus,
          secondFocus,
          nestedFocus,
          readonlyFocus,
        ]) {
          node.dispose();
        }
        for (final controller in [first, second, nested, readonly]) {
          controller.dispose();
        }
      });
      var edits = 0;
      var enabled = true;
      late StateSetter setLocalState;
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xff000000),
          builder: (_, _) => StatefulBuilder(
            builder: (_, setState) {
              setLocalState = setState;
              return AutofillGroup(
                child: Column(
                  children: [
                    editor(first, firstFocus, const ['email']),
                    editor(
                      second,
                      secondFocus,
                      enabled ? const ['username'] : null,
                      onChanged: (_) => edits++,
                    ),
                    AutofillGroup(
                      child: editor(nested, nestedFocus, const ['nested']),
                    ),
                    editor(readonly, readonlyFocus, const [
                      'readonly',
                    ], readOnly: true),
                  ],
                ),
              );
            },
          ),
        ),
      );

      nestedFocus.requestFocus();
      await tester.pump();
      final nestedTag = fieldTag(lastConfiguration(tester), 'nested');

      firstFocus.requestFocus();
      await tester.pump();
      final configuration = lastConfiguration(tester);
      final secondTag = fieldTag(configuration, 'username');
      final readonlyTag = fieldTag(configuration, 'readonly');

      expect((configuration['fields']! as List<Object?>).length, 3);
      expect(second.selection.baseOffset, -1);
      const newValue = TextEditingValue(
        text: 'A😃z',
        selection: TextSelection(
          baseOffset: 4,
          extentOffset: 1,
          affinity: TextAffinity.upstream,
          isDirectional: true,
        ),
        composing: TextRange(start: 1, end: 3),
      );
      await fill(tester, {
        secondTag: newValue,
        nestedTag: const TextEditingValue(text: 'outside'),
        readonlyTag: const TextEditingValue(text: 'write denied'),
      });
      expect(second.value, newValue.copyWith(composing: TextRange.empty));
      expect(nested.text, 'nested');
      expect(readonly.text, 'read only');
      expect(firstFocus.hasFocus, isTrue);
      expect(secondFocus.hasFocus, isFalse);
      expect(edits, 1);

      await fill(tester, {
        secondTag: const TextEditingValue(text: 'new second'),
      });
      expect(second.selection.baseOffset, -1);
      setLocalState(() => enabled = false);
      await tester.pump();
      await fill(tester, {secondTag: const TextEditingValue(text: 'stale')});
      expect(second.text, 'new second');
      expect(edits, 2);
    },
  );
}
