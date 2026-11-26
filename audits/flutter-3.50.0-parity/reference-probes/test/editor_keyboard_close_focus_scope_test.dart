import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('keyboard close focuses the scope, not a previously focused editor', (tester) async {
    final scope = FocusScopeNode();
    final first = FocusNode();
    final second = FocusNode();
    final a = TextEditingController(text: 'one');
    final b = TextEditingController(text: 'two');
    addTearDown(scope.dispose);
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    addTearDown(a.dispose);
    addTearDown(b.dispose);
    Widget editor(FocusNode focus, TextEditingController controller) => SizedBox(
      height: 40,
      child: EditableText(
        controller: controller,
        focusNode: focus,
        style: const TextStyle(fontSize: 14),
        cursorColor: const Color(0xFF000000),
        backgroundCursorColor: const Color(0xFF333333),
      ),
    );
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: FocusScope(node: scope, child: Column(children: [editor(first, a), editor(second, b)])),
      ),
    );
    first.requestFocus();
    await tester.pump();
    expect(first.hasPrimaryFocus, isTrue);
    second.requestFocus();
    await tester.pump();
    expect(second.hasPrimaryFocus, isTrue);
    expect(scope.focusedChild, second);

    tester.testTextInput.log.clear();
    tester.testTextInput.closeConnection();
    await tester.pump();

    expect(first.hasFocus, isFalse);
    expect(second.hasFocus, isFalse);
    expect(scope.hasPrimaryFocus, isTrue);
    expect(scope.focusedChild, isNull);
    expect(tester.testTextInput.log, isEmpty);

    first.requestFocus();
    await tester.pump();
    expect(first.hasPrimaryFocus, isTrue);
    expect(scope.hasPrimaryFocus, isFalse);
    expect(tester.testTextInput.editingState?['text'], 'one');
  });
}
