import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('replacement keeps active composition and adjusts the range', (
    WidgetTester tester,
  ) async {
    final controller = TextEditingController.fromValue(
      const TextEditingValue(
        text: 'tset next',
        selection: TextSelection(
          baseOffset: 4,
          extentOffset: 0,
          affinity: TextAffinity.upstream,
          isDirectional: true,
        ),
        composing: TextRange(start: 0, end: 4),
      ),
    );
    final focus = FocusNode();
    final changes = <String>[];
    var formats = 0;
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: MediaQuery(
          data: const MediaQueryData(),
          child: EditableText(
            controller: controller,
            focusNode: focus,
            style: const TextStyle(),
            cursorColor: const Color(0xFF000000),
            backgroundCursorColor: const Color(0xFF000000),
            onChanged: changes.add,
            inputFormatters: [
              TextInputFormatter.withFunction((_, next) {
                formats++;
                return next;
              }),
            ],
          ),
        ),
      ),
    );
    final context = tester.element(
      find
          .descendant(
            of: find.byType(EditableText),
            matching: find.byType(CompositedTransformTarget),
          )
          .last,
    );
    void replace(String text, TextRange range) {
      Actions.invoke(
        context,
        ReplaceTextIntent(
          controller.value,
          text,
          range,
          SelectionChangedCause.toolbar,
        ),
      );
    }

    const word = TextRange(start: 0, end: 4);
    replace('test', word);
    expect(controller.text, 'test next');
    expect(controller.selection.baseOffset, 4);
    expect(controller.selection.extentOffset, 0);
    expect(controller.selection.affinity, TextAffinity.upstream);
    expect(controller.selection.isDirectional, true);
    expect(controller.value.composing, word);
    await tester.pump();
    expect(focus.hasFocus, isFalse);

    replace('ok', word);
    await tester.pump();
    expect(controller.text, 'ok next');
    expect(focus.hasFocus, isTrue);
    expect(
      controller.selection,
      const TextSelection(baseOffset: 2, extentOffset: 0),
    );
    expect(controller.value.composing, const TextRange(start: 0, end: 2));
    expect(changes, ['test next', 'ok next']);
    expect(formats, 2);

    replace('ok', const TextRange(start: 0, end: 2));
    expect(formats, 2);
  });

  testWidgets('an identical replacement requests focus without formatting', (
    WidgetTester tester,
  ) async {
    final controller = TextEditingController(text: 'éx');
    final focus = FocusNode();
    var formats = 0;
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
                builder: (_) => EditableText(
                  controller: controller,
                  focusNode: focus,
                  style: const TextStyle(),
                  cursorColor: const Color(0xFF000000),
                  backgroundCursorColor: const Color(0xFF000000),
                  inputFormatters: [
                    TextInputFormatter.withFunction((_, next) {
                      formats++;
                      return next;
                    }),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
    expect(focus.hasFocus, isFalse);
    final context = tester.element(find.byType(CompositedTransformTarget));
    Actions.invoke(
      context,
      ReplaceTextIntent(
        controller.value,
        'é',
        const TextRange(start: 0, end: 1),
        SelectionChangedCause.toolbar,
      ),
    );
    await tester.pump();
    expect(focus.hasFocus, isTrue);
    expect(formats, 0);
    expect(controller.text, 'éx');
  });
}
