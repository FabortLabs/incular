import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final maintain in [false, true]) {
    testWidgets('Nested SafeArea consumes bottom padding: maintain=$maintain', (
      t,
    ) async {
      const key = ValueKey('probe');
      await t.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: SizedBox(
              width: 200,
              height: 200,
              child: MediaQuery(
                data: const MediaQueryData(
                  padding: EdgeInsets.only(bottom: 20),
                  viewPadding: EdgeInsets.only(bottom: 20),
                ),
                child: SafeArea(
                  maintainBottomViewPadding: maintain,
                  child: const SafeArea(child: SizedBox.expand(key: key)),
                ),
              ),
            ),
          ),
        ),
      );
      expect(t.getSize(find.byKey(key)).height, 180);
    });
  }
  testWidgets('Hidden retained editor stops being the active input client', (
    t,
  ) async {
    final focus = FocusNode();
    final c = TextEditingController(text: 'a');
    Widget scene(bool visible) => Directionality(
      textDirection: TextDirection.ltr,
      child: Visibility(
        visible: visible,
        maintainState: true,
        child: EditableText(
          controller: c,
          focusNode: focus,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xff000000),
          backgroundCursorColor: const Color(0xff888888),
        ),
      ),
    );
    try {
      await t.pumpWidget(scene(true));
      await t.showKeyboard(find.byType(EditableText));
      expect(focus.hasFocus, isTrue);
      await t.pumpWidget(scene(false));
      await t.pump();
      expect(focus.hasFocus, isFalse);
      expect(c.text, 'a');
    } finally {
      await t.pumpWidget(const SizedBox());
      focus.dispose();
      c.dispose();
    }
  });
}
