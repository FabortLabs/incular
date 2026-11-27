import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('selection callback precedes text callback in one platform edit', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController(text: 'hi');
    final FocusNode focus = FocusNode();
    final List<String> calls = <String>[];
    bool restoreText = false;
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    controller.selection = const TextSelection.collapsed(offset: 2);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          autofocus: true,
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xffffffff),
          backgroundCursorColor: const Color(0xff333333),
          onSelectionChanged: (selection, cause) {
            expect(cause, SelectionChangedCause.keyboard);
            calls.add('selection:${selection.extentOffset}');
            if (restoreText) {
              controller.value = const TextEditingValue(
                text: 'hello',
                selection: TextSelection.collapsed(offset: 5),
              );
            }
          },
          onChanged: (text) => calls.add('text:$text'),
        ),
      ),
    );
    await tester.pump();
    calls.clear();

    tester.testTextInput.updateEditingValue(
      const TextEditingValue(text: 'hello', selection: TextSelection.collapsed(offset: 5)),
    );
    await tester.pump();
    expect(calls, <String>['selection:5', 'text:hello']);

    restoreText = true;
    calls.clear();
    tester.testTextInput.updateEditingValue(
      const TextEditingValue(text: 'hello!', selection: TextSelection.collapsed(offset: 6)),
    );
    await tester.pump();
    expect(controller.text, 'hello');
    expect(calls, <String>['selection:6']);
  });
}
