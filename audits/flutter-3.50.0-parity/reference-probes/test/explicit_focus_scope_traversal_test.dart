import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('An explicit parent controls scope traversal and eligibility', (
    WidgetTester tester,
  ) async {
    final left = FocusScopeNode(debugLabel: 'left');
    final right = FocusScopeNode(debugLabel: 'right');
    final leaf = FocusNode(debugLabel: 'leaf');
    for (final node in [left, right, leaf]) {
      addTearDown(node.dispose);
    }

    Widget scene(FocusNode? parent) => Directionality(
      textDirection: TextDirection.ltr,
      child: Row(
        children: [
          FocusScope.withExternalFocusNode(
            focusScopeNode: left,
            child: const SizedBox(width: 50, height: 50),
          ),
          FocusScope.withExternalFocusNode(
            focusScopeNode: right,
            child: Focus.withExternalFocusNode(
              focusNode: leaf,
              parentNode: parent,
              child: const SizedBox(width: 50, height: 50),
            ),
          ),
        ],
      ),
    );

    await tester.pumpWidget(scene(left));
    expect(left.traversalChildren, [leaf]);
    expect(right.traversalDescendants, isEmpty);
    right.descendantsAreFocusable = false;
    right.descendantsAreTraversable = false;
    expect(leaf.canRequestFocus, isTrue);
    expect(left.traversalDescendants, [leaf]);
    left.requestFocus(leaf);
    await tester.pump();
    expect(leaf.hasPrimaryFocus, isTrue);
    expect(left.focusedChild, leaf);
    expect(right.hasFocus, isFalse);

    left.descendantsAreTraversable = false;
    expect(left.traversalDescendants, isEmpty);
    expect(leaf.canRequestFocus, isTrue);
    left.descendantsAreTraversable = true;
    right.descendantsAreFocusable = true;
    right.descendantsAreTraversable = true;
    await tester.pumpWidget(scene(null));
    expect(leaf.hasPrimaryFocus, isTrue);
    expect(left.traversalDescendants, isEmpty);
    expect(right.traversalChildren, [leaf]);
    expect(left.focusedChild, isNull);
    expect(right.focusedChild, leaf);
    expect(left.hasFocus, isFalse);
    expect(right.hasFocus, isTrue);
  });
}
