import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a cancelled touch cannot retarget the active editor', (
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
    Widget editor(TextEditingController controller, FocusNode focus) =>
        SizedBox(
          width: 180,
          height: 40,
          child: EditableText(
            controller: controller,
            focusNode: focus,
            style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xff808080),
          ),
        );
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [editor(a, first), editor(b, second)],
          ),
        ),
      ),
    );
    final bottom = find.byType(EditableText).last;
    await tester.tap(find.byType(EditableText).first);
    await tester.pump();
    expect(first.hasPrimaryFocus, true);
    final oldSelection = b.selection;
    final gesture = await tester.startGesture(
      tester.getCenter(bottom),
      kind: PointerDeviceKind.touch,
    );
    await tester.pump(const Duration(milliseconds: 16));
    expect(first.hasPrimaryFocus, true);
    expect(second.hasPrimaryFocus, false);
    expect(b.selection, oldSelection);
    await gesture.cancel();
    await tester.pump();
    expect(first.hasPrimaryFocus, true);
    tester.testTextInput.enterText('ax');
    await tester.pump();
    expect((a.text, b.text), ('ax', 'b'));

    await tester.tap(bottom);
    await tester.pump();
    expect(second.hasPrimaryFocus, true);
  }, variant: TargetPlatformVariant.only(TargetPlatform.android));
}
