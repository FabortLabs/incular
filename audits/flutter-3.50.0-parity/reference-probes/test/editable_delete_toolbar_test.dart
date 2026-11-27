import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _HandleControls extends EmptyTextSelectionControls
    with TextSelectionHandleControls {}

void main() {
  const toolbar = ValueKey<String>('editing toolbar');

  for (final (key, caret, remaining, visible) in [
    (LogicalKeyboardKey.backspace, 0, 'abc', true),
    (LogicalKeyboardKey.delete, 3, 'abc', true),
    (LogicalKeyboardKey.delete, 1, 'ac', false),
  ]) {
    testWidgets('${key.keyLabel} at $caret leaves toolbar visible: $visible', (
      tester,
    ) async {
      final controller = TextEditingController(text: 'abc')
        ..selection = TextSelection.collapsed(offset: caret == 0 ? 1 : 0);
      final focus = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(focus.dispose);
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xff303030),
          pageRouteBuilder:
              <T>(RouteSettings settings, WidgetBuilder builder) =>
                  PageRouteBuilder<T>(
                    settings: settings,
                    pageBuilder: (context, _, _) => builder(context),
                  ),
          home: Center(
            child: SizedBox(
              width: 300,
              child: EditableText(
                autofocus: true,
                controller: controller,
                focusNode: focus,
                style: const TextStyle(fontSize: 14),
                cursorColor: const Color(0xff000000),
                backgroundCursorColor: const Color(0xff999999),
                selectionControls: _HandleControls(),
                contextMenuBuilder: (context, state) =>
                    const SizedBox(key: toolbar, width: 40, height: 20),
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      expect(focus.hasFocus, isTrue);
      final editor = tester.state<EditableTextState>(find.byType(EditableText));
      editor.userUpdateTextEditingValue(
        controller.value.copyWith(
          selection: TextSelection.collapsed(offset: caret),
        ),
        SelectionChangedCause.tap,
      );
      await tester.pump();
      expect(editor.showToolbar(), isTrue);
      await tester.pump();
      expect(find.byKey(toolbar), findsOneWidget);

      await tester.sendKeyEvent(key);
      await tester.pump();
      expect(controller.text, remaining);
      expect(controller.selection, TextSelection.collapsed(offset: caret));
      expect(focus.hasFocus, isTrue);
      expect(find.byKey(toolbar), visible ? findsOneWidget : findsNothing);
    });
  }
}
