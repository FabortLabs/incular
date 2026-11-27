import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool directional in <bool>[false, true]) {
    testWidgets(
      '${directional ? 'directional' : 'sequential'} traversal reveals only the near edge',
      (WidgetTester tester) async {
        final ScrollController scroll = ScrollController();
        final FocusScopeNode scope = FocusScopeNode();
        final List<FocusNode> nodes = List<FocusNode>.generate(
          3,
          (_) => FocusNode(),
        );
        addTearDown(scroll.dispose);
        for (final FocusNode node in <FocusNode>[scope, ...nodes]) {
          addTearDown(node.dispose);
        }
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Center(
              child: SizedBox(
                width: 120,
                height: 120,
                child: SingleChildScrollView(
                  controller: scroll,
                  child: FocusScope(
                    node: scope,
                    child: Column(
                      children: <Widget>[
                        for (final FocusNode node in nodes)
                          Focus(
                            focusNode: node,
                            child: const SizedBox(width: 120, height: 70),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        nodes[0].requestFocus();
        await tester.pump();
        expect(nodes[0].hasPrimaryFocus, isTrue);
        expect(scroll.offset, 0);

        for (final (bool reverse, int expected, double offset)
            in <(bool, int, double)>[
              (false, 1, 20),
              (false, 2, 90),
              (true, 1, 70),
              (true, 0, 0),
            ]) {
          bool moved;
          if (directional) {
            moved = scope.focusInDirection(
              reverse ? TraversalDirection.up : TraversalDirection.down,
            );
          } else {
            moved = reverse ? scope.previousFocus() : scope.nextFocus();
          }
          expect(moved, isTrue);
          await tester.pumpAndSettle();
          expect(nodes[expected].hasPrimaryFocus, isTrue);
          expect(scroll.offset, offset);
        }
      },
    );
  }
}
