import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('focus-excluded editor does not interrupt active IME', (
    tester,
  ) async {
    final a = TextEditingController(text: 'a');
    final b = TextEditingController(text: 'b');
    final first = FocusNode();
    final second = FocusNode();
    addTearDown(a.dispose);
    addTearDown(b.dispose);
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    Widget editor(TextEditingController controller, FocusNode node) => SizedBox(
      width: 180,
      height: 40,
      child: EditableText(
        controller: controller,
        focusNode: node,
        style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
        cursorColor: const Color(0xff000000),
        backgroundCursorColor: const Color(0xff808080),
      ),
    );
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Column(
          children: [
            editor(a, first),
            ExcludeFocus(child: editor(b, second)),
          ],
        ),
      ),
    );
    first.requestFocus();
    await tester.pump();
    const composing = TextRange(start: 1, end: 2);
    tester.testTextInput.updateEditingValue(
      const TextEditingValue(
        text: 'aあ',
        selection: TextSelection.collapsed(offset: 2),
        composing: composing,
      ),
    );
    await tester.pump();
    expect(a.value.composing, composing);
    tester.testTextInput.log.clear();

    final gesture = await tester.startGesture(
      tester.getCenter(find.byType(EditableText).last),
      kind: PointerDeviceKind.mouse,
    );
    await tester.pump();
    expect(first.hasPrimaryFocus, true);
    expect(a.value.composing, composing);
    await gesture.up();
    await tester.pump();
    expect((first.hasPrimaryFocus, second.hasPrimaryFocus), (true, false));
    expect(a.value.composing, composing);
    expect(
      tester.testTextInput.log.where((call) =>
          call.method == 'TextInput.clearClient' ||
          call.method == 'TextInput.setClient' ||
          call.method == 'TextInput.hide'),
      isEmpty,
    );
    tester.testTextInput.updateEditingValue(
      const TextEditingValue(
        text: 'a界',
        selection: TextSelection.collapsed(offset: 2),
      ),
    );
    await tester.pump();
    expect((a.text, b.text), ('a界', 'b'));
  });
}
