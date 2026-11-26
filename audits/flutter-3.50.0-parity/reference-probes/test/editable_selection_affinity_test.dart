import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'an affinity-only collapsed selection update moves the caret between bidi edges',
    (tester) async {
      final controller = TextEditingController(text: 'hi שלום bye');
      final focus = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(focus.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: SizedBox(
              width: 300,
              height: 32,
              child: EditableText(
                controller: controller,
                focusNode: focus,
                showCursor: false,
                style: const TextStyle(fontSize: 14),
                cursorColor: const Color(0xffff0000),
                backgroundCursorColor: const Color(0xff000000),
              ),
            ),
          ),
        ),
      );
      // UTF-16 offset 7 == native UTF-8 byte 11 in this exact string.
      final values = <TextAffinity>[];
      controller.addListener(() => values.add(controller.selection.affinity));
      double x() {
        final editable = tester.allRenderObjects
            .whereType<RenderEditable>()
            .single;
        return editable.getLocalRectForCaret(controller.selection.extent).left;
      }

      controller.selection = const TextSelection.collapsed(
        offset: 7,
        affinity: TextAffinity.upstream,
      );
      await tester.pump();
      final upstream = x();
      controller.selection = const TextSelection.collapsed(offset: 7);
      await tester.pump();
      final downstream = x();
      expect(values, [TextAffinity.upstream, TextAffinity.downstream]);
      expect(upstream, lessThan(downstream - 5));
    },
  );
}
