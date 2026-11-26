import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class ExplicitChecker implements SpellCheckService {
  final calls = <String>[];

  @override
  Future<List<SuggestionSpan>?> fetchSpellCheckSuggestions(
    Locale locale,
    String text,
  ) async {
    calls.add(text);
    return const [];
  }
}

void main() {
  testWidgets('default native spelling is used only for an opted-in editor', (
    WidgetTester tester,
  ) async {
    final dispatcher = tester.binding.platformDispatcher;
    dispatcher.nativeSpellCheckServiceDefinedTestValue = true;
    addTearDown(dispatcher.clearNativeSpellCheckServiceDefined);
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    final wire = <List<Object?>>[];
    messenger.setMockMethodCallHandler(SystemChannels.spellCheck, (call) async {
      expect(call.method, 'SpellCheck.initiateSpellCheck');
      wire.add((call.arguments as List).cast<Object?>());
      return <Object?>[];
    });
    addTearDown(
      () => messenger.setMockMethodCallHandler(SystemChannels.spellCheck, null),
    );

    final custom = ExplicitChecker();
    final fields = <Widget>[];
    for (var index = 0; index < 3; index++) {
      final text = TextEditingController();
      final focus = FocusNode();
      addTearDown(text.dispose);
      addTearDown(focus.dispose);
      fields.add(
        EditableText(
          controller: text,
          focusNode: focus,
          obscureText: index == 2,
          locale: const Locale('en', 'GB'),
          style: const TextStyle(),
          cursorColor: const Color(0xFF000000),
          backgroundCursorColor: const Color(0xFF000000),
          spellCheckConfiguration: SpellCheckConfiguration(
            misspelledTextStyle: const TextStyle(),
            spellCheckService: index == 1 ? custom : null,
          ),
        ),
      );
    }
    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(),
        child: Directionality(
          textDirection: TextDirection.ltr,
          child: Column(children: fields),
        ),
      ),
    );

    for (var index = 0; index < fields.length; index++) {
      final state = tester.state<EditableTextState>(
        find.byType(EditableText).at(index),
      );
      expect(state.spellCheckEnabled, index < 2);
      final value = 'value-$index';
      state.userUpdateTextEditingValue(
        TextEditingValue(
          text: value,
          selection: TextSelection.collapsed(offset: value.length),
        ),
        SelectionChangedCause.keyboard,
      );
      await tester.pump();
    }
    expect(wire, [
      ['en-GB', 'value-0'],
    ]);
    expect(custom.calls, ['value-1']);
  });
}
