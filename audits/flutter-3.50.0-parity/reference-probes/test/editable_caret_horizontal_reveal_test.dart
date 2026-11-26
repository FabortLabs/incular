import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a fitting glyph still scrolls to fully reveal a wider caret', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetDevicePixelRatio);
    final controller = TextEditingController(text: 'A');
    final focus = FocusNode();
    final scroll = ScrollController();
    final key = GlobalKey<EditableTextState>();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    addTearDown(scroll.dispose);

    Future<void> layout(double width) => tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: width,
            height: 30,
            child: EditableText(
              key: key,
              controller: controller,
              focusNode: focus,
              scrollController: scroll,
              style: const TextStyle(fontSize: 14),
              cursorWidth: 4,
              cursorColor: const Color(0xff111f31),
              backgroundCursorColor: const Color(0xff000000),
            ),
          ),
        ),
      ),
    );
    const start = TextPosition(offset: 0);
    const end = TextPosition(offset: 1);
    controller.selection = const TextSelection.collapsed(offset: 1);
    await layout(500);
    final state = key.currentState!;
    final baseline = state.renderEditable.getLocalRectForCaret(end);
    expect(scroll.offset, 0);
    expect(state.renderEditable.maxScrollExtent, 0);

    final width = baseline.left + 3;
    await layout(width);
    expect(state.renderEditable.maxScrollExtent, 2);
    state.bringIntoView(end);
    await tester.pump();
    expect(scroll.offset, 1);
    expect(state.renderEditable.getLocalRectForCaret(end).right, width);

    state.bringIntoView(start);
    await tester.pump();
    expect(scroll.offset, 0);
    expect(state.renderEditable.getLocalRectForCaret(start).left, 0);
  });

  testWidgets(
    'a fitting right aligned editor reserves one pixel after the caret',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetDevicePixelRatio);
      final controller = TextEditingController(text: 'A');
      final focus = FocusNode();
      final key = GlobalKey<EditableTextState>();
      addTearDown(controller.dispose);
      addTearDown(focus.dispose);

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 80,
              height: 30,
              child: EditableText(
                key: key,
                controller: controller,
                focusNode: focus,
                textAlign: TextAlign.right,
                style: const TextStyle(fontSize: 14),
                cursorWidth: 4,
                cursorColor: const Color(0xff111f31),
                backgroundCursorColor: const Color(0xff000000),
              ),
            ),
          ),
        ),
      );

      final editable = key.currentState!.renderEditable;
      expect(editable.maxScrollExtent, 0);
      expect(
        editable.getLocalRectForCaret(const TextPosition(offset: 1)).right,
        79,
      );
    },
  );
}
