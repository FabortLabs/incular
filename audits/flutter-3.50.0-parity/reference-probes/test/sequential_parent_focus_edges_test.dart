import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'tab leaves the child scope and re-enters from the approached edge',
    (WidgetTester tester) async {
      final List<FocusNode> nodes = List<FocusNode>.generate(
        4,
        (_) => FocusNode(),
      );
      final FocusScopeNode inner = FocusScopeNode(
        traversalEdgeBehavior: TraversalEdgeBehavior.parentScope,
      );
      final FocusScopeNode outer = FocusScopeNode();
      for (final FocusNode node in <FocusNode>[...nodes, inner, outer]) {
        addTearDown(node.dispose);
      }
      Widget stop(int index) => Focus(
        focusNode: nodes[index],
        child: const SizedBox(width: 100, height: 100),
      );
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: FocusScope(
            node: outer,
            child: Column(
              children: <Widget>[
                stop(0),
                FocusScope(
                  node: inner,
                  child: Column(children: <Widget>[stop(1), stop(2)]),
                ),
                stop(3),
              ],
            ),
          ),
        ),
      );

      nodes[0].requestFocus();
      await tester.pump();
      for (final (bool reverse, int expected) in <(bool, int)>[
        (false, 1),
        (false, 2),
        (false, 3),
        (true, 2),
        (true, 1),
        (true, 0),
        (false, 1),
      ]) {
        final FocusNode primary = FocusManager.instance.primaryFocus!;
        expect(reverse ? primary.previousFocus() : primary.nextFocus(), isTrue);
        await tester.pump();
        expect(FocusManager.instance.primaryFocus, same(nodes[expected]));
      }
    },
  );
}
