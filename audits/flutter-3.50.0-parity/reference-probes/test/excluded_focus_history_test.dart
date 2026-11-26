import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('excluding the current branch restores the last eligible child', (
    tester,
  ) async {
    final scope = FocusScopeNode();
    final outside = FocusNode();
    final first = FocusNode();
    final second = FocusNode();
    addTearDown(() {
      scope.dispose();
      outside.dispose();
      first.dispose();
      second.dispose();
    });
    Widget scene(bool exclude) => FocusScope(
      node: scope,
      child: Column(
        textDirection: TextDirection.ltr,
        children: [
          Focus(focusNode: outside, child: const SizedBox()),
          ExcludeFocus(
            excluding: exclude,
            child: Column(
              children: [
                Focus(focusNode: first, child: const SizedBox()),
                Focus(focusNode: second, child: const SizedBox()),
              ],
            ),
          ),
        ],
      ),
    );
    await tester.pumpWidget(scene(false));
    for (final node in [outside, first, second]) {
      node.requestFocus();
      await tester.pump();
      expect(node.hasPrimaryFocus, isTrue);
    }
    await tester.pumpWidget(scene(true));
    await tester.pump();
    expect(outside.hasPrimaryFocus, isTrue);
    expect(first.hasFocus || second.hasFocus, isFalse);
    expect(scope.hasPrimaryFocus, isFalse);
    expect(scope.focusedChild, same(outside));
  });
}
