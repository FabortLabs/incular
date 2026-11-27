import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('the exact visual edge belongs to the next space', (
    tester,
  ) async {
    final controller = TextEditingController(text: '  test');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: EditableText(
            controller: controller,
            focusNode: focus,
            style: const TextStyle(fontSize: 10, height: 1),
            cursorColor: const Color(0xffff0000),
            backgroundCursorColor: const Color(0xffcccccc),
          ),
        ),
      ),
    );
    final editable = tester
        .state<EditableTextState>(find.byType(EditableText))
        .renderEditable;
    for (final (x, affinity) in const [
      (9.0, TextAffinity.upstream),
      (10.0, TextAffinity.downstream),
      (11.0, TextAffinity.downstream),
    ]) {
      expect(
        editable.getPositionForPoint(Offset(x, 2)),
        TextPosition(offset: 1, affinity: affinity),
      );
    }
  });
}
