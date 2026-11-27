import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'unkeyed semantics with no direction use paint order in both axes',
    (tester) async {
      Widget box(String id, double x, double y) => Positioned(
        top: y,
        left: x,
        child: Semantics(
          button: true,
          identifier: id,
          child: const SizedBox.square(dimension: 30),
        ),
      );
      await tester.pumpWidget(
        Stack(
          alignment: Alignment.center,
          children: [
            Semantics(
              button: true,
              identifier: 'placeholder',
              child: const Placeholder(),
            ),
            box('box-0', 100, 200),
            box('box-1', 200, 100),
            box('box-2', 100, 100),
            box('box-3', 0, 100),
            box('box-4', 100, 10),
          ],
        ),
      );

      final SemanticsNode root =
          tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
      final children = root.debugListChildrenInOrder(
        DebugSemanticsDumpOrder.traversalOrder,
      );
      expect(
        children.map(
          (SemanticsNode node) => node.getSemanticsData().identifier,
        ),
        ['placeholder', 'box-0', 'box-1', 'box-2', 'box-3', 'box-4'],
      );
    },
  );
}
