import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Changing an inactive scope priority does not steal focus', (tester) async {
    final parent = FocusScopeNode();
    final first = FocusScopeNode();
    final second = FocusScopeNode();
    final other = FocusNode();
    for (final node in [parent, first, second, other]) {
      addTearDown(node.dispose);
    }
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Column(
          children: [
            Focus(focusNode: other, autofocus: true, child: const SizedBox(width: 20, height: 20)),
            FocusScope(
              node: parent,
              child: Column(
                children: [
                  FocusScope(node: first, child: const SizedBox(width: 20, height: 20)),
                  FocusScope(node: second, child: const SizedBox(width: 20, height: 20)),
                ],
              ),
            ),
          ],
        ),
      ),
    );
    expect(other.hasPrimaryFocus, isTrue);
    parent.setFirstFocus(first);
    expect(first.isFirstFocus, isTrue);
    parent.setFirstFocus(second);
    expect(first.isFirstFocus, isFalse);
    expect(second.isFirstFocus, isTrue);
    await tester.pump();
    expect(other.hasPrimaryFocus, isTrue);
    expect(second.hasFocus, isFalse);

    parent.requestFocus();
    await tester.pump();
    expect(second.hasPrimaryFocus, isTrue);
    expect(other.hasFocus, isFalse);
  });
}
