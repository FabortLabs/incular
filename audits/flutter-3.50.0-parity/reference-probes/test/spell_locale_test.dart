import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Recorder implements SpellCheckService {
  final requests = <(Locale, String)>[];

  @override
  Future<List<SuggestionSpan>?> fetchSpellCheckSuggestions(
    Locale locale,
    String text,
  ) async {
    requests.add((locale, text));
    return null;
  }
}

void main() {
  testWidgets('spell checking follows the editor then the widget locale', (
    WidgetTester tester,
  ) async {
    final text = TextEditingController();
    final focus = FocusNode();
    final locale = ValueNotifier<Locale?>(const Locale('tr'));
    final inherited = ValueNotifier<Locale>(const Locale('en', 'GB'));
    final recorder = Recorder();
    final config = SpellCheckConfiguration(
      spellCheckService: recorder,
      misspelledTextStyle: const TextStyle(),
    );
    addTearDown(text.dispose);
    addTearDown(focus.dispose);
    addTearDown(locale.dispose);
    addTearDown(inherited.dispose);

    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(),
        child: ValueListenableBuilder<Locale>(
          valueListenable: inherited,
          builder: (_, fallback, _) => Localizations(
            locale: fallback,
            delegates: const [DefaultWidgetsLocalizations.delegate],
            child: ValueListenableBuilder<Locale?>(
              valueListenable: locale,
              builder: (_, override, _) => EditableText(
                controller: text,
                focusNode: focus,
                locale: override,
                hintLocales: const [Locale('de', 'DE')],
                style: const TextStyle(),
                cursorColor: const Color(0xFF000000),
                backgroundCursorColor: const Color(0xFF000000),
                spellCheckConfiguration: config,
              ),
            ),
          ),
        ),
      ),
    );
    final state = tester.state<EditableTextState>(find.byType(EditableText));
    void type(String value) {
      state.userUpdateTextEditingValue(
        TextEditingValue(
          text: value,
          selection: TextSelection.collapsed(offset: value.length),
        ),
        SelectionChangedCause.keyboard,
      );
    }

    type('Istanbul');
    await tester.pump();
    expect(state.renderEditable.locale, const Locale('tr'));
    locale.value = null;
    await tester.pump();
    type('colour');
    await tester.pump();
    inherited.value = const Locale('fr', 'CA');
    await tester.pump();
    type('couleur');
    await tester.pump();

    expect(recorder.requests, [
      (const Locale('tr'), 'Istanbul'),
      (const Locale('en', 'GB'), 'colour'),
      (const Locale('fr', 'CA'), 'couleur'),
    ]);
  });
}
