import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('touching suggestion endpoints use the lower midpoint first', (
    WidgetTester tester,
  ) async {
    final controller = TextEditingController();
    final focusNode = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focusNode.dispose);
    tester.binding.platformDispatcher.nativeSpellCheckServiceDefinedTestValue =
        true;
    addTearDown(
      tester.binding.platformDispatcher.clearNativeSpellCheckServiceDefined,
    );
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          controller: controller,
          focusNode: focusNode,
          style: const TextStyle(),
          cursorColor: const Color(0xFF000000),
          backgroundCursorColor: const Color(0xFF000000),
          spellCheckConfiguration: const SpellCheckConfiguration(
            misspelledTextStyle: TextStyle(),
          ),
        ),
      ),
    );
    final state = tester.state<EditableTextState>(find.byType(EditableText));
    const left = SuggestionSpan(TextRange(start: 0, end: 2), <String>['a']);
    const right = SuggestionSpan(TextRange(start: 2, end: 4), <String>['b']);
    state.spellCheckResults = const SpellCheckResults('abcd', [left, right]);
    expect(state.findSuggestionSpanAtCursorIndex(2), left);
    expect(state.findSuggestionSpanAtCursorIndex(4), right);
    expect(state.findSuggestionSpanAtCursorIndex(-1), isNull);
  });
}
