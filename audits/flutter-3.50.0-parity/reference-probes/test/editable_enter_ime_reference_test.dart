import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('raw Enter does not submit but IME Done completes then submits', (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final controller = TextEditingController();
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    final calls = <String>[];

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: DefaultTextEditingShortcuts(
          child: EditableText(
            controller: controller,
            focusNode: focus,
            style: const TextStyle(color: Color(0xFF000000)),
            cursorColor: const Color(0xFF000000),
            backgroundCursorColor: const Color(0xFF000000),
            onChanged: (value) => calls.add('changed:$value'),
            onEditingComplete: () => calls.add('complete'),
            onSubmitted: (value) => calls.add('submitted:$value'),
          ),
        ),
      ),
    );
    await tester.showKeyboard(find.byType(EditableText));
    expect(focus.hasFocus, isTrue);
    tester.testTextInput.enterText('ab');
    await tester.pump();
    expect(calls, ['changed:ab']);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();
    expect(calls, ['changed:ab']);
    expect(controller.text, 'ab');
    expect(focus.hasFocus, isTrue);

    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pump();
    expect(calls, ['changed:ab', 'complete', 'submitted:ab']);
    expect(controller.text, 'ab');
    expect(focus.hasFocus, isTrue);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}
