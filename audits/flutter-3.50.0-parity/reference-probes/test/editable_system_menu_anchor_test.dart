// Authored for Incular parity; not an upstream test registration.

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('default items and global text menu selection rectangle', (
    WidgetTester tester,
  ) async {
    final controller = TextEditingController(text: 'test\nwide');
    final focus = FocusNode();
    final key = GlobalKey<EditableTextState>();
    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFFFFFFFF),
        builder: (context, child) => Align(
          alignment: Alignment.topLeft,
          child: Padding(
            padding: const EdgeInsets.only(left: 13, top: 17),
            child: SizedBox(
              width: 400,
              child: EditableText(
                key: key,
                controller: controller,
                focusNode: focus,
                autofocus: true,
                maxLines: 10,
                textAlign: TextAlign.right,
                style: const TextStyle(fontFamily: 'Ahem', fontSize: 10),
                cursorColor: const Color(0xFF0000FF),
                backgroundCursorColor: const Color(0xFF808080),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(focus.hasPrimaryFocus, isTrue);
    final state = key.currentState!;
    expect(SystemContextMenu.getDefaultItems(state), [
      const IOSSystemContextMenuItemCut(),
      const IOSSystemContextMenuItemCopy(),
      const IOSSystemContextMenuItemPaste(),
      const IOSSystemContextMenuItemSelectAll(),
    ]);
    for (final (selection, expected) in [
      (const TextSelection.collapsed(offset: 2), (390.0, 17.0, 0.0, 10.0)),
      (
        const TextSelection(baseOffset: 1, extentOffset: 3),
        (380.0, 17.0, 20.0, 10.0),
      ),
      (
        const TextSelection(baseOffset: 1, extentOffset: 7),
        (13.0, 17.0, 400.0, 20.0),
      ),
      (
        const TextSelection(baseOffset: 7, extentOffset: 1),
        (13.0, 17.0, 400.0, 20.0),
      ),
    ]) {
      controller.selection = selection;
      await tester.pump();
      final glyphs = state.getGlyphHeights();
      final render = state.renderEditable;
      final endpoints = render.getEndpointsForSelection(selection);
      final anchor = TextSelectionToolbarAnchors.getSelectionRect(
        render,
        glyphs.startGlyphHeight,
        glyphs.endGlyphHeight,
        endpoints,
      );
      expect(anchor.left, closeTo(expected.$1, 0.002));
      expect(anchor.top, closeTo(expected.$2, 0.002));
      expect(anchor.width, closeTo(expected.$3, 0.002));
      expect(anchor.height, closeTo(expected.$4, 0.002));
    }
    await tester.pumpWidget(const SizedBox.shrink());
    controller.dispose();
    focus.dispose();
  });
}
