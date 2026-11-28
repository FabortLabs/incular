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
  Offset getHandleAnchor(TextSelectionHandleType type, double textLineHeight) =>
      Offset.zero;

  @override
  Size getHandleSize(double textLineHeight) => Size.zero;
}

void main() {
  testWidgets('a custom editor toolbar receives the full overlay constraints', (
    tester,
  ) async {
    final editing = TextEditingController(text: 'text');
    final focus = FocusNode();
    addTearDown(editing.dispose);
    addTearDown(focus.dispose);
    const menu = Key('ten pixel toolbar');
    await tester.pumpWidget(
      MaterialApp(
        home: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 400,
            height: 100,
            child: EditableText(
              controller: editing,
              focusNode: focus,
              selectionControls: _Handles(),
              style: const TextStyle(),
              cursorColor: const Color(0xff000000),
              backgroundCursorColor: const Color(0xff000000),
              contextMenuBuilder: (_, _) =>
                  const SizedBox(key: menu, width: 10, height: 10),
            ),
          ),
        ),
      ),
    );
    await tester.showKeyboard(find.byType(EditableText));
    tester.testTextInput.updateEditingValue(
      editing.value.copyWith(
        selection: const TextSelection(baseOffset: 0, extentOffset: 4),
      ),
    );
    await tester.pumpAndSettle();
    final state = tester.state<EditableTextState>(find.byType(EditableText));
    expect(state.showToolbar(), isTrue);
    await tester.pumpAndSettle();
    expect(tester.getSize(find.byKey(menu)), MediaQuery.sizeOf(state.context));
  });
}
