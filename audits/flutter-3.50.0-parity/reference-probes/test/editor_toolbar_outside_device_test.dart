import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart' show MaterialApp;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Handles extends TextSelectionControls with TextSelectionHandleControls {
  @override
  Widget buildHandle(
    BuildContext context,
    TextSelectionHandleType type,
    double textLineHeight, [
    VoidCallback? onTap,
  ]) => const SizedBox.shrink();

  @override
  Offset getHandleAnchor(TextSelectionHandleType type, double textLineHeight) => Offset.zero;

  @override
  Size getHandleSize(double textLineHeight) => Size.zero;
}

void main() {
  testWidgets('an internal scroll waits to restore a selection hidden by device padding', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(400, 200);
    addTearDown(tester.view.reset);

    final editing = TextEditingController.fromValue(
      TextEditingValue(
        text: List<String>.filled(20, 'abcd').join('\n'),
        selection: const TextSelection.collapsed(offset: 0),
      ),
    );
    final focus = FocusNode();
    final scroll = ScrollController();
    addTearDown(editing.dispose);
    addTearDown(focus.dispose);
    addTearDown(scroll.dispose);
    const toolbar = Key('toolbar');
    await tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(size: Size(400, 200)),
          child: EditableText(
            focusNode: focus,
            controller: editing,
            scrollController: scroll,
            maxLines: null,
            selectionControls: _Handles(),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xff000000),
            style: const TextStyle(),
            contextMenuBuilder: (_, _) => const SizedBox.shrink(key: toolbar),
          ),
        ),
      ),
    );
    await tester.showKeyboard(find.byType(EditableText));
    tester.testTextInput.updateEditingValue(editing.value.copyWith(
      selection: const TextSelection(baseOffset: 50, extentOffset: 54),
    ));
    await tester.pumpAndSettle();
    final state = tester.state<EditableTextState>(find.byType(EditableText));
    expect(state.showToolbar(), isTrue);
    await tester.pumpAndSettle();
    expect(find.byKey(toolbar), findsOneWidget);
    tester.view.padding = const FakeViewPadding(bottom: 120);
    await tester.pump();

    final selectionCaret = state.renderEditable.getLocalRectForCaret(const TextPosition(offset: 50));
    final globalCaret = state.renderEditable.localToGlobal(selectionCaret.topLeft);
    expect(View.of(state.context).padding.bottom, 120);
    expect(globalCaret.dy, greaterThan(80), reason: 'selection stays outside drawable view');
    expect(scroll.position.maxScrollExtent, greaterThan(10));

    Future<void> drag(double direction) async {
      final start = tester.getCenter(find.byType(EditableText));
      final gesture = await tester.startGesture(start);
      await gesture.moveBy(Offset(0, direction * 30));
      await tester.pump(const Duration(milliseconds: 50));
      await gesture.moveBy(Offset(0, direction * 10));
      await tester.pump(const Duration(milliseconds: 50));
      expect(scroll.position.isScrollingNotifier.value, isTrue);
      expect(find.byKey(toolbar), findsNothing);
      await gesture.up();
      await tester.pumpAndSettle();
    }

    await drag(-1);
    expect(scroll.offset, greaterThan(0));
    expect(editing.selection, const TextSelection(baseOffset: 50, extentOffset: 54));
    expect(find.byKey(toolbar), findsNothing);
    tester.view.padding = const FakeViewPadding();
    await tester.pump();
    await drag(1);
    expect(find.byKey(toolbar), findsOneWidget);

    await tester.pumpWidget(const SizedBox.shrink());
  }, variant: const TargetPlatformVariant(<TargetPlatform>{TargetPlatform.android}));
}
