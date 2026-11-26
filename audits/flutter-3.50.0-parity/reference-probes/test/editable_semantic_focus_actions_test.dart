import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'editable advertises selection and text mutation only while focused',
    (tester) async {
      final focus = FocusNode();
      final controller = TextEditingController.fromValue(
        const TextEditingValue(
          text: 'hello',
          selection: TextSelection(baseOffset: 1, extentOffset: 3),
        ),
      );
      addTearDown(focus.dispose);
      addTearDown(controller.dispose);

      Future<void> mount({required bool readOnly}) async {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: EditableText(
              controller: controller,
              focusNode: focus,
              readOnly: readOnly,
              selectAllOnFocus: false,
              showCursor: false,
              style: const TextStyle(),
              cursorColor: const Color(0xff000000),
              backgroundCursorColor: const Color(0xff000000),
            ),
          ),
        );
      }

      void check({
        required bool focused,
        required bool setSelection,
        required bool setText,
      }) {
        final editable = tester.allRenderObjects
            .whereType<RenderEditable>()
            .single;
        // Match the native fixture by exercising the retained render object directly.
        editable.hasFocus = focused;
        final result = SemanticsConfiguration();
        editable.describeSemanticsConfiguration(result);
        expect(
          result.textSelection,
          const TextSelection(baseOffset: 1, extentOffset: 3),
        );
        expect(
          result.getActionHandler(ui.SemanticsAction.setSelection) != null,
          setSelection,
        );
        expect(
          result.getActionHandler(ui.SemanticsAction.setText) != null,
          setText,
        );
      }

      await mount(readOnly: false);
      check(focused: false, setSelection: false, setText: false);
      check(focused: true, setSelection: true, setText: true);
      await mount(readOnly: true);
      check(focused: true, setSelection: true, setText: false);
      check(focused: false, setSelection: false, setText: false);
    },
  );
}
