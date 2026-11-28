import 'package:flutter/rendering.dart' show RenderEditable;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'raw editor recomputes line height with ambient and explicit behavior',
    (WidgetTester tester) async {
      final controller = TextEditingController(text: 'A');
      final focus = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(focus.dispose);
      const inherited = TextHeightBehavior(applyHeightToFirstAscent: false);
      const explicit = TextHeightBehavior(
        applyHeightToFirstAscent: false,
        applyHeightToLastDescent: false,
      );
      const normal = TextHeightBehavior();
      RenderEditable? retained;
      for (final (ambient, override, expectedHeight) in [
        (inherited, null, 25.0),
        (inherited, explicit, 20.0),
        (normal, null, 40.0),
        (inherited, null, 25.0),
      ]) {
        await tester.pumpWidget(
          MediaQuery(
            data: const MediaQueryData(),
            child: Directionality(
              textDirection: TextDirection.ltr,
              child: Center(
                child: DefaultTextHeightBehavior(
                  textHeightBehavior: ambient,
                  child: EditableText(
                    controller: controller,
                    focusNode: focus,
                    style: const TextStyle(
                      fontFamily: 'FlutterTest',
                      fontSize: 20,
                      height: 2,
                    ),
                    cursorColor: const Color(0xFF2196F3),
                    backgroundCursorColor: const Color(0xFF9E9E9E),
                    strutStyle: StrutStyle.disabled,
                    textHeightBehavior: override,
                  ),
                ),
              ),
            ),
          ),
        );
        final finder = find.byType(EditableText);
        final state = tester.state<EditableTextState>(finder);
        final renderer = state.renderEditable;
        expect(renderer, same(retained ??= renderer));
        expect(renderer.textHeightBehavior, override ?? ambient);
        expect(renderer.size.height, expectedHeight);
      }
      await tester.pumpWidget(const SizedBox());
    },
  );
}
