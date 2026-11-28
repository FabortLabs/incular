import 'dart:ui';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('undefined view re-entry restores an editor and its selection', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'abcdef');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFFFFFFFF),
        builder: (_, _) => EditableText(
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xFF111111),
          backgroundCursorColor: const Color(0xFFFFFFFF),
        ),
      ),
    );
    focus.requestFocus();
    await tester.pump();
    const selection = TextSelection(baseOffset: 4, extentOffset: 1);
    controller.selection = selection;
    await tester.pump();
    for (final state in [ViewFocusState.unfocused, ViewFocusState.focused]) {
      ServicesBinding.instance.platformDispatcher.onViewFocusChange?.call(
        ViewFocusEvent(
          viewId: tester.view.viewId,
          direction: ViewFocusDirection.undefined,
          state: state,
        ),
      );
      await tester.pump();
      expect(focus.hasPrimaryFocus, state == ViewFocusState.focused);
      expect(controller.selection, selection);
    }
  });
}
