import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

class _FixedControls extends TextSelectionControls
    with TextSelectionHandleControls {
  _FixedControls(this.size, this.anchor);

  final Size size;
  final Offset anchor;

  @override
  Widget buildHandle(
    BuildContext context,
    TextSelectionHandleType type,
    double textLineHeight, [
    VoidCallback? onTap,
  ]) => SizedBox.fromSize(size: size);

  @override
  Size getHandleSize(double textLineHeight) => size;

  @override
  Offset getHandleAnchor(TextSelectionHandleType type, double textLineHeight) =>
      anchor;
}

void main() {
  testWidgets(
    'Caret reveal reserves the actual handle clearance in a sliver group',
    (WidgetTester tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(800, 600);
      addTearDown(tester.view.reset);
      for (final (controls, bottom) in <(TextSelectionControls?, double)>[
        (null, 20),
        (_FixedControls(Size.zero, Offset.zero), 24),
        (_FixedControls(const Size(22, 22), const Offset(11, -4)), 39),
        (_FixedControls(const Size(50, 60), const Offset(25, 6)), 54),
      ]) {
        final editing = TextEditingController();
        final focus = FocusNode();
        final scroll = ScrollController();
        addTearDown(editing.dispose);
        addTearDown(focus.dispose);
        addTearDown(scroll.dispose);

        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: CustomScrollView(
                controller: scroll,
                slivers: [
                  SliverMainAxisGroup(
                    slivers: [
                      const SliverToBoxAdapter(child: SizedBox(height: 600)),
                      SliverToBoxAdapter(
                        child: SizedBox(
                          height: 44,
                          child: EditableText(
                            controller: editing,
                            focusNode: focus,
                            selectionControls: controls,
                            style: const TextStyle(fontSize: 14),
                            cursorColor: const Color(0xffffffff),
                            backgroundCursorColor: const Color(0xff808080),
                          ),
                        ),
                      ),
                      const SliverToBoxAdapter(child: SizedBox(height: 500)),
                    ],
                  ),
                ],
              ),
            ),
          ),
        );
        expect(scroll.offset, 0);
        await tester.showKeyboard(
          find.byType(EditableText, skipOffstage: false),
        );
        await tester.pumpAndSettle();
        expect(focus.hasPrimaryFocus, isTrue);
        final editor = tester.state<EditableTextState>(
          find.byType(EditableText),
        );
        final caret = editor.renderEditable.getLocalRectForCaret(
          const TextPosition(offset: 0),
        );
        expect(scroll.offset, caret.bottom + bottom);
        await tester.pumpWidget(const SizedBox());
      }
    },
  );
}
