import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('selection endpoints are local and include the paint offset', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'ab');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: Transform.translate(
            offset: const Offset(30, 25),
            child: SizedBox(
              width: 100,
              child: EditableText(
                controller: controller,
                focusNode: focus,
                style: const TextStyle(fontSize: 10, height: 1),
                cursorColor: const Color(0xffff0000),
                backgroundCursorColor: const Color(0xffcccccc),
              ),
            ),
          ),
        ),
      ),
    );
    final editable = tester
        .state<EditableTextState>(find.byType(EditableText))
        .renderEditable;
    editable.offset = ViewportOffset.fixed(5);
    await tester.pump();
    const range = <TextSelectionPoint>[
      TextSelectionPoint(Offset(-5, 10), TextDirection.ltr),
      TextSelectionPoint(Offset(15, 10), TextDirection.ltr),
    ];
    expect(
      editable.getEndpointsForSelection(
        const TextSelection(baseOffset: 0, extentOffset: 2),
      ),
      range,
    );
    expect(
      editable.getEndpointsForSelection(
        const TextSelection(baseOffset: 2, extentOffset: 0),
      ),
      range,
    );
    expect(
      editable.getEndpointsForSelection(
        const TextSelection.collapsed(offset: 1),
      ),
      const [TextSelectionPoint(Offset(5, 10), null)],
    );
  });
}
