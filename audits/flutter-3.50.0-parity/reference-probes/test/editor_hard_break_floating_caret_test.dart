import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('floating caret starts on the new line and lands on the explicit prior line end', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController(
      text: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ\n1234567890',
    );
    final FocusNode focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    EditableText.debugDeterministicCursor = true;
    addTearDown(() => EditableText.debugDeterministicCursor = false);
    controller.selection = const TextSelection.collapsed(offset: 27);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: EditableText(
            autofocus: true,
            controller: controller,
            focusNode: focus,
            maxLines: 2,
            style: const TextStyle(fontSize: 14),
            cursorColor: const Color(0xff111ffb),
            backgroundCursorColor: const Color(0xff333333),
          ),
        ),
      ),
    );
    await tester.pump();
    final EditableTextState state = tester.state(find.byType(EditableText));
    final RenderEditable render = state.renderEditable;
    void send(FloatingCursorDragState phase, [Offset offset = Offset.zero]) {
      state.updateFloatingCursor(RawFloatingCursorPoint(state: phase, offset: offset));
    }

    send(FloatingCursorDragState.Start);
    await tester.pump();
    expect(
      render,
      paints..rrect(rrect: RRect.fromRectAndRadius(const Rect.fromLTWH(0.5, 15, 3, 12), const Radius.circular(1))),
    );
    send(FloatingCursorDragState.Update, const Offset(9999, -14));
    await tester.pump();
    expect(
      render,
      paints..rrect(rrect: RRect.fromRectAndRadius(const Rect.fromLTWH(800.5, 1, 3, 12), const Radius.circular(1))),
    );
    send(FloatingCursorDragState.End);
    await tester.pump();
    expect(controller.selection.baseOffset, 27, reason: 'reset has not elapsed');
    await tester.pump(const Duration(milliseconds: 130));
    expect(controller.selection.baseOffset, 26);
    expect(controller.selection.affinity, TextAffinity.upstream);
  });
}
