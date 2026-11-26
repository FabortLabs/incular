import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'focused lazy editor survives a scroll and is evicted after host close',
    (tester) async {
      final scroll = ScrollController();
      final focus = FocusNode();
      final text = TextEditingController(text: 'draft');
      final key = GlobalKey<EditableTextState>();
      addTearDown(scroll.dispose);
      addTearDown(focus.dispose);
      addTearDown(text.dispose);

      await tester.pumpWidget(
        MediaQuery(
          data: const MediaQueryData(),
          child: Directionality(
            textDirection: TextDirection.ltr,
            child: ListView.builder(
              controller: scroll,
              itemCount: 60,
              itemExtent: 40,
              itemBuilder: (_, index) => index == 0
                  ? EditableText(
                      key: key,
                      controller: text,
                      focusNode: focus,
                      autofocus: true,
                      style: const TextStyle(fontSize: 14),
                      cursorColor: const Color(0xff000000),
                      backgroundCursorColor: const Color(0xff000000),
                    )
                  : const SizedBox.expand(),
            ),
          ),
        ),
      );
      await tester.pump();
      final state = key.currentState!;
      expect(focus.hasPrimaryFocus, isTrue);
      expect(state.wantKeepAlive, isTrue);

      scroll.jumpTo(scroll.position.maxScrollExtent);
      await tester.pump();
      expect(key.currentState, same(state));
      expect(focus.hasPrimaryFocus, isTrue);

      tester.testTextInput.log.clear();
      tester.testTextInput.closeConnection();
      await tester.pumpAndSettle();
      expect(focus.hasFocus, isFalse);
      expect(key.currentState, isNull);
      expect(text.text, 'draft');
      expect(tester.testTextInput.log, isEmpty);
    },
  );
}
