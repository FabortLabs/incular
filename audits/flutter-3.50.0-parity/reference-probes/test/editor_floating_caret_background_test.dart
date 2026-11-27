import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('floating caret paints a distant text anchor only while dragging', (
    WidgetTester tester,
  ) async {
    const Color cursor = Color(0xff111ffb);
    const Color background = Color(0xff875634);
    final TextEditingController controller = TextEditingController(text: 'AAAA');
    final FocusNode focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    EditableText.debugDeterministicCursor = true;
    addTearDown(() => EditableText.debugDeterministicCursor = false);

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          autofocus: true,
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 80),
          cursorColor: cursor,
          backgroundCursorColor: background,
        ),
      ),
    );
    controller.selection = const TextSelection.collapsed(offset: 0);
    await tester.pump();
    final EditableTextState state = tester.state(find.byType(EditableText));
    final RenderEditable editable = state.renderEditable;
    final Color floating = cursor.withOpacity(0.75);
    void drag(FloatingCursorDragState phase, [double x = 0]) {
      state.updateFloatingCursor(
        RawFloatingCursorPoint(state: phase, offset: Offset(x, 0)),
      );
    }

    drag(FloatingCursorDragState.Start);
    await tester.pump();
    expect(editable, paints..rrect(color: floating));
    expect(editable, isNot(paints..rect(color: background)));
    drag(FloatingCursorDragState.Update, 10);
    await tester.pump();
    expect(editable, isNot(paints..rect(color: background)));
    drag(FloatingCursorDragState.Update, 25);
    await tester.pump();
    expect(
      editable,
      paints
        ..rect(color: background, rect: editable.getLocalRectForCaret(const TextPosition(offset: 0)))
        ..rrect(color: floating),
    );
    expect(controller.selection.baseOffset, 0);

    drag(FloatingCursorDragState.End);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 20));
    expect(editable, paints..rrect(color: floating));
    expect(editable, isNot(paints..rect(color: background)));
    await tester.pump(const Duration(milliseconds: 120));
    expect(editable, isNot(paints..rrect(color: floating)));
    expect(editable, paints..rect(color: cursor));
  });
}
