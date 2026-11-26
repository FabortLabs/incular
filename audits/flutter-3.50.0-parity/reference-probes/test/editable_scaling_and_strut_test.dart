import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('EditableText scales the authored strut using the explicit scaler, not its ambient scale', (
    WidgetTester tester,
  ) async {
    final controller = TextEditingController(text: 'Hello World');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);

    Widget editor(double ambient, {TextScaler? override, StrutStyle? strut}) => MediaQuery(
      data: MediaQueryData(textScaler: TextScaler.linear(ambient)),
      child: Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: SizedBox(
            width: 200,
            child: EditableText(
              controller: controller,
              focusNode: focus,
              style: const TextStyle(fontFamily: 'Ahem', fontSize: 10),
              textScaler: override,
              strutStyle: strut,
              showCursor: false,
              cursorColor: const Color(0),
              backgroundCursorColor: const Color(0),
            ),
          ),
        ),
      ),
    );
    final finder = find.byType(EditableText);

    await tester.pumpWidget(editor(3));
    final retained = tester.element(finder);
    expect(tester.getSize(finder).height, 30);
    const force = StrutStyle(fontSize: 50, forceStrutHeight: true);
    await tester.pumpWidget(editor(3, override: const TextScaler.linear(2), strut: force));
    expect(tester.element(finder), same(retained));
    expect(tester.getSize(finder).height, 100);
    await tester.pumpWidget(editor(4, override: const TextScaler.linear(2), strut: force));
    expect(tester.getSize(finder).height, 100);
    await tester.pumpWidget(editor(4));
    expect(tester.getSize(finder).height, 40);
  });
}
