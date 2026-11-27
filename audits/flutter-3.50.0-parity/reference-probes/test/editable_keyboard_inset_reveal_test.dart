import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('An inset increase jumps the caret into view but a decrease does not', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(800, 600);
    addTearDown(tester.view.reset);
    final controller = TextEditingController();
    final focus = FocusNode();
    final scroll = ScrollController();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    addTearDown(scroll.dispose);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            controller: scroll,
            child: Column(
              children: [
                for (var index = 0; index < 5; index += 1)
                  const SizedBox(height: 1200),
                SizedBox(
                  height: 20,
                  child: EditableText(
                    controller: controller,
                    focusNode: focus,
                    style: const TextStyle(),
                    cursorColor: const Color(0xffffffff),
                    backgroundCursorColor: const Color(0xff808080),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.showKeyboard(find.byType(EditableText));
    await tester.pumpAndSettle();
    expect(focus.hasPrimaryFocus, isTrue);
    scroll.jumpTo(0);
    await tester.pumpAndSettle();
    expect(scroll.offset, 0);
    final selection = controller.selection;

    tester.view.viewInsets = const FakeViewPadding(bottom: 500);
    await tester.pump();
    expect(scroll.offset, 5920);
    expect(scroll.offset, scroll.position.maxScrollExtent);
    expect(controller.selection, selection);

    scroll.jumpTo(0);
    await tester.pumpAndSettle();
    tester.view.viewInsets = const FakeViewPadding(bottom: 400);
    await tester.pump();
    expect(scroll.offset, 0);
    expect(focus.hasPrimaryFocus, isTrue);
  });
}
