import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('showOnScreen reveals the part under a fixed pinned header', (
    tester,
  ) async {
    final controller = ScrollController();
    final target = GlobalKey();
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          controller: controller,
          slivers: [
            const PinnedHeaderSliver(child: SizedBox(height: 200)),
            const SliverToBoxAdapter(child: SizedBox(height: 300)),
            SliverToBoxAdapter(child: SizedBox(key: target, height: 100)),
            const SliverToBoxAdapter(child: SizedBox(height: 1200)),
          ],
        ),
      ),
    );
    controller.jumpTo(475);
    await tester.pump();
    target.currentContext!.findRenderObject()!.showOnScreen();
    await tester.pump();
    expect(controller.offset, 300);
    await tester.pumpWidget(const SizedBox());
    controller.dispose();
  });

  testWidgets('focusing an editor reveals its caret past a pinned header', (
    tester,
  ) async {
    final controller = ScrollController();
    final editor = TextEditingController(text: 'Hello');
    final focus = FocusNode();
    final key = GlobalKey<EditableTextState>();
    await tester.pumpWidget(
      MediaQuery(
        data: MediaQueryData.fromView(tester.view),
        child: Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            controller: controller,
            slivers: [
              const PinnedHeaderSliver(child: SizedBox(height: 200)),
              const SliverToBoxAdapter(child: SizedBox(height: 300)),
              SliverToBoxAdapter(
                child: SizedBox(
                  height: 100,
                  child: EditableText(
                    key: key,
                    controller: editor,
                    focusNode: focus,
                    style: const TextStyle(
                      fontSize: 14,
                      color: Color(0xff000000),
                    ),
                    cursorColor: const Color(0xff000000),
                    backgroundCursorColor: const Color(0xff000000),
                    showCursor: false,
                    scrollPadding: EdgeInsets.zero,
                  ),
                ),
              ),
              const SliverToBoxAdapter(child: SizedBox(height: 1200)),
            ],
          ),
        ),
      ),
    );
    controller.jumpTo(475);
    await tester.pump();
    focus.requestFocus();
    await tester.pumpAndSettle();
    final box = key.currentState!.renderEditable;
    final caret = box.localToGlobal(
      box
          .getLocalRectForCaret(
            TextPosition(offset: editor.selection.extentOffset),
          )
          .topLeft,
    );
    expect(focus.hasPrimaryFocus, isTrue);
    expect(controller.offset, lessThan(475));
    expect(caret.dy, greaterThanOrEqualTo(199.999));
    await tester.pumpWidget(const SizedBox());
    controller.dispose();
    editor.dispose();
    focus.dispose();
  });
}
