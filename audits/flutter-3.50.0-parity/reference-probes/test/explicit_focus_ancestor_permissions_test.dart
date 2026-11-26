import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Mounted node permissions come from logical, not visual, focus ancestors', (
    WidgetTester tester,
  ) async {
    final alternate = FocusNode(debugLabel: 'alternate');
    final visual = FocusNode(debugLabel: 'visual');
    final leaf = FocusNode(debugLabel: 'leaf');
    for (final node in [alternate, visual, leaf]) {
      addTearDown(node.dispose);
    }
    visual.descendantsAreFocusable = false;
    visual.descendantsAreTraversable = false;

    Widget scene(FocusNode? explicit) => FocusScope(
      child: Column(
        children: [
          Focus.withExternalFocusNode(focusNode: alternate, child: const SizedBox()),
          Focus.withExternalFocusNode(
            focusNode: visual,
            child: Focus.withExternalFocusNode(
              focusNode: leaf,
              parentNode: explicit,
              child: const SizedBox(),
            ),
          ),
        ],
      ),
    );

    await tester.pumpWidget(scene(alternate));
    expect(leaf.canRequestFocus, isTrue);
    expect(leaf.skipTraversal, isFalse);
    alternate.descendantsAreTraversable = false;
    expect(leaf.skipTraversal, isTrue);
    expect(leaf.canRequestFocus, isTrue);
    leaf.requestFocus();
    await tester.pump();
    expect(leaf.hasPrimaryFocus, isTrue);

    alternate.descendantsAreFocusable = false;
    expect(leaf.canRequestFocus, isFalse);
    alternate.requestFocus();
    await tester.pump();
    leaf.requestFocus();
    await tester.pump();
    expect(alternate.hasPrimaryFocus, isTrue);
    expect(leaf.hasPrimaryFocus, isFalse);

    alternate.descendantsAreFocusable = true;
    alternate.descendantsAreTraversable = true;
    expect(leaf.canRequestFocus, isTrue);
    expect(leaf.skipTraversal, isFalse);
    await tester.pump();
    expect(alternate.hasPrimaryFocus, isTrue);

    await tester.pumpWidget(scene(null));
    expect(leaf.canRequestFocus, isFalse);
    expect(leaf.skipTraversal, isTrue);
  });
}
