import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final (readOnly, obscure, interactive, allowed) in [
    (true, true, null, false),
    (true, true, true, false),
    (true, false, false, true),
    (false, true, false, true),
  ]) {
    testWidgets(
      'explicit selectAll, readOnly=$readOnly obscure=$obscure interactive=$interactive',
      (tester) async {
        final controller = TextEditingController(text: 'blah blah');
        final focus = FocusNode();
        addTearDown(controller.dispose);
        addTearDown(focus.dispose);
        controller.selection = const TextSelection.collapsed(offset: 2);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: EditableText(
              controller: controller,
              focusNode: focus,
              backgroundCursorColor: const Color(0xFFAAAAAA),
              cursorColor: const Color(0xFF000000),
              style: const TextStyle(),
              readOnly: readOnly,
              obscureText: obscure,
              enableInteractiveSelection: interactive,
            ),
          ),
        );
        final state = tester.state<EditableTextState>(
          find.byType(EditableText),
        );
        expect(focus.hasFocus, isFalse);
        state.selectAll(SelectionChangedCause.toolbar);
        await tester.pump();
        expect(
          controller.selection,
          allowed
              ? const TextSelection(baseOffset: 0, extentOffset: 9)
              : const TextSelection.collapsed(offset: 2),
        );
        expect(focus.hasFocus, allowed);
        if (!allowed) expect(state.selectAllEnabled, isFalse);
      },
    );
  }
}
