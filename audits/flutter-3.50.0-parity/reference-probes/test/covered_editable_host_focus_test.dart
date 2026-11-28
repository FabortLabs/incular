import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'The pinned Flutter still accepts a host refocus request behind a route',
    (tester) async {
      final navigatorKey = GlobalKey<NavigatorState>();
      final controller = TextEditingController(text: 'kept');
      final focus = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(focus.dispose);
      const ink = Color(0xFF222222);
      await tester.pumpWidget(
        WidgetsApp(
          navigatorKey: navigatorKey,
          color: ink,
          pageRouteBuilder: <T>(settings, builder) => PageRouteBuilder<T>(
            settings: settings,
            pageBuilder: (context, animation, secondaryAnimation) =>
                Builder(builder: builder),
          ),
          home: EditableText(
            controller: controller,
            focusNode: focus,
            style: const TextStyle(color: ink, fontSize: 14),
            cursorColor: ink,
            backgroundCursorColor: ink,
            autofocus: true,
          ),
        ),
      );
      await tester.pump();
      final editor = tester.state<EditableTextState>(find.byType(EditableText));
      expect(focus.hasPrimaryFocus, isTrue);
      editor.connectionClosed();
      await tester.pump();
      expect(focus.hasFocus, isFalse);

      final pushed = navigatorKey.currentState!.push<void>(
        PageRouteBuilder<void>(
          transitionDuration: Duration.zero,
          reverseTransitionDuration: Duration.zero,
          pageBuilder: (context, animation, secondaryAnimation) =>
              const SizedBox(width: 200, height: 200),
        ),
      );
      await tester.pumpAndSettle();
      expect(editor.mounted, isTrue);
      // The pinned original is skipped and this is the divergence it hides.
      expect(editor.onFocusReceived(), isTrue);
      await tester.pump();

      navigatorKey.currentState!.pop();
      await tester.pumpAndSettle();
      await pushed;
      expect(controller.text, 'kept');
    },
  );
}
