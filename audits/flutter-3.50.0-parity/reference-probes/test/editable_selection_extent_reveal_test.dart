import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Selected glyph geometry determines the horizontal reveal at either extent', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(800, 600);
    addTearDown(tester.view.reset);
    final editing = TextEditingController(text: 'AAAAAAAAAAAAAAAAAAAA')
      ..selection = const TextSelection.collapsed(offset: 0);
    final focus = FocusNode();
    final scroll = ScrollController();
    addTearDown(editing.dispose);
    addTearDown(focus.dispose);
    addTearDown(scroll.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 100,
              height: 40,
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                controller: scroll,
                child: SizedBox(
                  width: 220,
                  child: EditableText(
                    controller: editing,
                    focusNode: focus,
                    autofocus: true,
                    selectAllOnFocus: false,
                    style: const TextStyle(fontSize: 10, height: 1),
                    cursorColor: const Color(0xffffffff),
                    backgroundCursorColor: const Color(0xff808080),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.showKeyboard(find.byType(EditableText));
    await tester.pumpAndSettle();
    expect(focus.hasPrimaryFocus, isTrue);
    expect(scroll.offset, 0);
    final editable = tester.state<EditableTextState>(find.byType(EditableText));

    // A single selection box wider than the viewport already covers it.
    editing.selection = const TextSelection(baseOffset: 0, extentOffset: 18);
    await tester.pumpAndSettle();
    tester.view.viewInsets = const FakeViewPadding(bottom: 50);
    await tester.pumpAndSettle();
    expect(scroll.offset, 0);

    editing.selection = const TextSelection(baseOffset: 16, extentOffset: 18);
    await tester.pumpAndSettle();
    tester.view.viewInsets = const FakeViewPadding(bottom: 100);
    await tester.pumpAndSettle();
    final glyphs = editable.renderEditable.getBoxesForSelection(editing.selection);
    expect(glyphs.single.left, 160);
    expect(glyphs.single.right, 180);
    expect(scroll.offset, 100);

    editing.selection = const TextSelection(baseOffset: 2, extentOffset: 0);
    await tester.pumpAndSettle();
    tester.view.viewInsets = const FakeViewPadding(bottom: 200);
    await tester.pumpAndSettle();
    expect(scroll.offset, 0);
    expect(focus.hasPrimaryFocus, isTrue);
  });
}
