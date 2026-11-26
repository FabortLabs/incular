import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'scope traversal snapshots keep descendant order and distinguish the two skip flags',
    (WidgetTester tester) async {
      final scope = FocusScopeNode();
      final parent1 = FocusNode();
      final parent2 = FocusNode();
      final child1 = FocusNode();
      final child2 = FocusNode();
      for (final node in [scope, parent1, parent2, child1, child2]) {
        addTearDown(node.dispose);
      }
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: FocusScope.withExternalFocusNode(
            focusScopeNode: scope,
            child: Column(
              children: [
                Focus.withExternalFocusNode(
                  focusNode: parent1,
                  child: Column(
                    children: [
                      Focus.withExternalFocusNode(
                        focusNode: child1,
                        child: const SizedBox.shrink(),
                      ),
                      Focus.withExternalFocusNode(
                        focusNode: child2,
                        child: const SizedBox.shrink(),
                      ),
                    ],
                  ),
                ),
                Focus.withExternalFocusNode(
                  focusNode: parent2,
                  child: const SizedBox.shrink(),
                ),
              ],
            ),
          ),
        ),
      );
      final root = scope.parent! as FocusScopeNode;
      final children = [child1, child2, parent1, parent2];
      child1.requestFocus();
      await tester.pump();

      expect(root.traversalDescendants, [...children, scope]);
      expect(scope.traversalDescendants, children);
      expect(child1.hasPrimaryFocus, isTrue);

      scope.skipTraversal = true;
      await tester.pump();
      expect(root.traversalDescendants, children);
      expect(scope.traversalDescendants, children);
      expect(scope.focusedChild, child1);

      scope.skipTraversal = false;
      scope.descendantsAreTraversable = false;
      await tester.pump();
      expect(root.traversalDescendants, [scope]);
      expect(scope.traversalDescendants, isEmpty);
      expect(scope.focusedChild, child1);
      child2.requestFocus();
      await tester.pump();
      expect(child2.hasPrimaryFocus, isTrue);
    },
  );
}
