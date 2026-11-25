import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('owning Focus callback does not flap between focused children', (
    tester,
  ) async {
    final first = FocusNode();
    final second = FocusNode();
    final outside = FocusNode();
    for (final node in [first, second, outside]) {
      addTearDown(node.dispose);
    }
    final states = <bool>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Column(
          children: [
            Focus(
              onFocusChange: states.add,
              child: Column(
                children: [
                  Focus(focusNode: first, child: const SizedBox()),
                  Focus(focusNode: second, child: const SizedBox()),
                ],
              ),
            ),
            Focus(focusNode: outside, child: const SizedBox()),
          ],
        ),
      ),
    );
    first.requestFocus();
    await tester.pump();
    second.requestFocus();
    await tester.pump();
    expect(states, [true]);
    outside.requestFocus();
    await tester.pump();
    expect(states, [true, false]);
  });

  testWidgets(
    'omitted permission preserves a node policy and explicit true reenables',
    (tester) async {
      final node = FocusNode(canRequestFocus: false);
      addTearDown(node.dispose);
      await tester.pumpWidget(Focus(focusNode: node, child: const SizedBox()));
      expect(node.canRequestFocus, isFalse);
      await tester.pumpWidget(
        Focus(focusNode: node, canRequestFocus: true, child: const SizedBox()),
      );
      expect(node.canRequestFocus, isTrue);
      expect(node.hasPrimaryFocus, isFalse);
      node.requestFocus();
      await tester.pump();
      expect(node.hasPrimaryFocus, isTrue);
    },
  );
  testWidgets('unmounted Focus no longer observes a reused external node', (
    tester,
  ) async {
    final node = FocusNode();
    addTearDown(node.dispose);
    final first = <bool>[];
    final second = <bool>[];
    await tester.pumpWidget(
      Focus(focusNode: node, onFocusChange: first.add, child: const SizedBox()),
    );
    node.requestFocus();
    await tester.pump();
    expect(first, [true]);
    await tester.pumpWidget(const SizedBox());
    expect(node.hasPrimaryFocus, isFalse);
    expect(first, [true]);
    await tester.pumpWidget(
      Focus(
        focusNode: node,
        onFocusChange: second.add,
        child: const SizedBox(),
      ),
    );
    node.requestFocus();
    await tester.pump();
    expect(first, [true]);
    expect(second, [true]);
  });
}
