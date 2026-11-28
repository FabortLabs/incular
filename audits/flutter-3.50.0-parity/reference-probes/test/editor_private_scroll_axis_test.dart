import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a short, overlong horizontal drag scrolls a single-line Android editor without focusing it',
    (tester) async {
      final text = TextEditingController(
        text: 'This is a long string that should overflow the TextField.',
      );
      final focus = FocusNode();
      addTearDown(text.dispose);
      addTearDown(focus.dispose);
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xff000000),
          builder: (_, _) => Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 100,
              child: EditableText(
                controller: text,
                focusNode: focus,
                style: const TextStyle(fontFamily: 'FlutterTest', fontSize: 14),
                cursorColor: const Color(0xff000000),
                backgroundCursorColor: const Color(0xff000000),
              ),
            ),
          ),
        ),
      );
      final finder = find.byType(EditableText);
      final position = tester
          .state<ScrollableState>(find.byType(Scrollable))
          .position;
      expect(position.pixels, 0);
      expect(position.maxScrollExtent, greaterThan(100));
      await tester.drag(finder, const Offset(-100, 0));
      await tester.pumpAndSettle();
      expect(position.pixels, greaterThan(0));
      expect(focus.hasPrimaryFocus, isFalse);
      expect(text.selection.isValid, isFalse);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.android),
  );
}
