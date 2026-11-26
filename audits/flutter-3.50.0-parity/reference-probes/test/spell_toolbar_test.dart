import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Checker implements SpellCheckService {
  @override
  Future<List<SuggestionSpan>?> fetchSpellCheckSuggestions(
    Locale locale,
    String text,
  ) async => const [];
}

void main() {
  testWidgets('a neutral editor opens a custom suggestion toolbar', (
    WidgetTester tester,
  ) async {
    final controller = TextEditingController(text: 'tset again');
    final focus = FocusNode();
    final changes = <String>[];
    final configuration = SpellCheckConfiguration(
      spellCheckService: _Checker(),
      misspelledTextStyle: const TextStyle(
        decoration: TextDecoration.underline,
      ),
      spellCheckSuggestionsToolbarBuilder: (context, state) {
        final span = state.findSuggestionSpanAtCursorIndex(
          state.textEditingValue.selection.extentOffset,
        )!;
        return Align(
          alignment: Alignment.topRight,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () {
              state.userUpdateTextEditingValue(
                state.textEditingValue.replaced(span.range, 'test'),
                SelectionChangedCause.toolbar,
              );
              state.hideToolbar();
            },
            child: Text(span.suggestions.first),
          ),
        );
      },
    );
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: MediaQuery(
          data: const MediaQueryData(),
          child: Overlay(
            initialEntries: [
              OverlayEntry(
                builder: (_) => Align(
                  alignment: Alignment.topLeft,
                  child: EditableText(
                    controller: controller,
                    focusNode: focus,
                    style: const TextStyle(),
                    cursorColor: const Color(0xFF000000),
                    backgroundCursorColor: const Color(0xFF000000),
                    contextMenuBuilder: (_, _) => const SizedBox.shrink(),
                    onChanged: changes.add,
                    locale: const Locale('en'),
                    spellCheckConfiguration: configuration,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
    final state = tester.state<EditableTextState>(find.byType(EditableText));
    expect(state.showSpellCheckSuggestionsToolbar(), isFalse);
    focus.requestFocus();
    await tester.pump();
    expect(focus.hasFocus, isTrue);
    expect(state.showSpellCheckSuggestionsToolbar(), isFalse);
    state.spellCheckResults = const SpellCheckResults('tset again', [
      SuggestionSpan(TextRange(start: 0, end: 4), ['test', 'sets', 'set']),
    ]);
    controller.selection = const TextSelection.collapsed(offset: 6);
    await tester.pump();
    expect(state.showSpellCheckSuggestionsToolbar(), isFalse);

    controller.value = controller.value.copyWith(
      selection: const TextSelection.collapsed(offset: 2),
      composing: const TextRange(start: 0, end: 4),
    );
    await tester.pump();
    expect(state.showSpellCheckSuggestionsToolbar(), isTrue);
    await tester.pump();
    expect(find.text('test'), findsOneWidget);
    await tester.tap(find.text('test'));
    await tester.pump();
    expect(controller.text, 'test again');
    expect(controller.selection, const TextSelection.collapsed(offset: 2));
    expect(controller.value.composing, const TextRange(start: 0, end: 4));
    expect(changes, ['test again']);
    expect(find.text('test'), findsNothing);
    expect(focus.hasFocus, isTrue);
  });
}
