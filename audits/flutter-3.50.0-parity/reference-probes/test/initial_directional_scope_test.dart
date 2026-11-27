import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('The nested scope is not preferred on equal edges or when skipped', (tester) async {
    for (final skipScope in [false, true]) {
      final outer = FocusScopeNode();
      final inner = FocusScopeNode(skipTraversal: skipScope);
      final leaf = FocusNode();
      addTearDown(outer.dispose);
      addTearDown(inner.dispose);
      addTearDown(leaf.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: FocusScope(
            node: outer,
            child: FocusScope(
              node: inner,
              skipTraversal: skipScope,
              child: Padding(
                padding: EdgeInsets.only(top: skipScope ? 10 : 0),
                child: Focus(focusNode: leaf, child: const SizedBox(width: 24, height: 24)),
              ),
            ),
          ),
        ),
      );
      expect(outer.focusInDirection(TraversalDirection.down), isTrue);
      await tester.pump();
      expect(leaf.hasPrimaryFocus, isTrue);
      expect(inner.hasPrimaryFocus, isFalse);
      await tester.pumpWidget(const SizedBox());
    }
  });

  testWidgets('With no eligible descendants, the initiating scope can receive focus', (tester) async {
    final outer = FocusScopeNode();
    addTearDown(outer.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: FocusScope(node: outer, child: const SizedBox(width: 24, height: 24)),
      ),
    );
    expect(outer.focusInDirection(TraversalDirection.down), isTrue);
    await tester.pump();
    expect(outer.hasPrimaryFocus, isTrue);
  });
}
