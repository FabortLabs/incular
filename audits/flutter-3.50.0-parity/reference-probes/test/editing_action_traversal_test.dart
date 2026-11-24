import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Next and Previous traverse only with default completion', (
    tester,
  ) async {
    for (final reverse in [false, true]) {
      for (final custom in [false, true]) {
        final start = reverse ? 1 : 0;
        final controllers = ['left', 'right'].map((text) =>
          TextEditingController(text: text)).toList();
        final nodes = List.generate(2, (_) => FocusNode());
        for (final item in controllers) {
          addTearDown(item.dispose);
        }
        for (final item in nodes) {
          addTearDown(item.dispose);
        }
        final calls = <String>[];
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Column(
              children: [
                for (final i in [0, 1])
                  SizedBox(
                    width: 180,
                    height: 40,
                    child: EditableText(
                      controller: controllers[i],
                      focusNode: nodes[i],
                      style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
                      cursorColor: const Color(0xff000000),
                      backgroundCursorColor: const Color(0xff808080),
                      onEditingComplete: custom && i == start
                          ? () => calls.add('complete')
                          : null,
                      onSubmitted: (text) => calls.add('submit:$text'),
                    ),
                  ),
              ],
            ),
          ),
        );
        await tester.showKeyboard(find.byType(EditableText).at(start));
        final text = controllers[start].text;
        const marked = TextRange(start: 0, end: 2);
        tester.testTextInput.updateEditingValue(
          TextEditingValue(
            text: text,
            selection: TextSelection.collapsed(offset: text.length),
            composing: marked,
          ),
        );
        await tester.pump();
        expect(controllers[start].value.composing, marked);
        await tester.testTextInput.receiveAction(
          reverse ? TextInputAction.previous : TextInputAction.next,
        );
        await tester.pump();
        expect(calls, [if (custom) 'complete', 'submit:$text']);
        expect(nodes[custom ? start : 1 - start].hasPrimaryFocus, true);
        expect(controllers[start].value.composing, custom ? marked : TextRange.empty);
        await tester.pumpWidget(const SizedBox.shrink());
      }
    }
  });
}
