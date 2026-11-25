import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('default password mask and caret positions use UTF-16 length', (
    WidgetTester tester,
  ) async {
    final password = TextEditingController(text: 'é👩‍💻');
    final bullets = TextEditingController(text: '••••••');
    final first = FocusNode();
    final second = FocusNode();
    for (final resources in [password, bullets, first, second]) {
      addTearDown(resources.dispose);
    }
    const passwordKey = ValueKey<String>('password');
    const bulletsKey = ValueKey<String>('bullets');
    Widget editor(
      Key key,
      TextEditingController controller,
      FocusNode node,
      bool obscure,
    ) => SizedBox(
      width: 240,
      height: 50,
      child: EditableText(
        key: key,
        controller: controller,
        focusNode: node,
        obscureText: obscure,
        style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
        cursorColor: const Color(0xff000000),
        backgroundCursorColor: const Color(0xff808080),
      ),
    );
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: Column(
            children: [
              editor(passwordKey, password, first, true),
              editor(bulletsKey, bullets, second, false),
            ],
          ),
        ),
      ),
    );
    final obscured = tester.state<EditableTextState>(find.byKey(passwordKey));
    final plain = tester.state<EditableTextState>(find.byKey(bulletsKey));
    expect(password.text.length, 6);
    expect(obscured.buildTextSpan().toPlainText(), '••••••');
    final RenderEditable secret = obscured.renderEditable;
    final RenderEditable glyphs = plain.renderEditable;
    for (final column in [0, 1, 3, 4, 6]) {
      final caret = TextPosition(offset: column);
      expect(
        secret.getLocalRectForCaret(caret),
        glyphs.getLocalRectForCaret(caret),
      );
    }

    password.value = const TextEditingValue(
      text: 'é👩‍💻💻',
      selection: TextSelection.collapsed(offset: 6),
      composing: TextRange(start: 1, end: 6),
    );
    await tester.pump();
    expect(obscured.buildTextSpan().toPlainText(), '••••••••');
  }, variant: const TargetPlatformVariant({TargetPlatform.linux}));
  testWidgets('a retained editor can replace its mask with a BMP character', (
    WidgetTester tester,
  ) async {
    final controller = TextEditingController(text: 'é👩‍💻');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    EditableText field(String glyph) => EditableText(
      controller: controller,
      focusNode: focus,
      obscureText: true,
      obscuringCharacter: glyph,
      style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
      cursorColor: const Color(0xff000000),
      backgroundCursorColor: const Color(0xff808080),
    );
    EditableTextState? retained;
    for (final glyph in ['#', 'é', '•']) {
      await tester.pumpWidget(
        Directionality(textDirection: TextDirection.ltr, child: field(glyph)),
      );
      final state = tester.state<EditableTextState>(find.byType(EditableText));
      retained ??= state;
      expect(state, same(retained));
      expect(state.buildTextSpan().toPlainText(), glyph * 6);
      expect(controller.text, 'é👩‍💻');
    }
    expect(() => field('🙂'), throwsAssertionError);
  });
}
