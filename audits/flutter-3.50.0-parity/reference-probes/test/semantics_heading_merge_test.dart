import 'package:flutter/rendering.dart' show RendererBinding, SemanticsNode;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('retained heading prefers parent then non-null child', (
    tester,
  ) async {
    const cases = [
      (null, null, 0),
      (null, 1, 1),
      (null, 2, 2),
      (1, null, 1),
      (2, null, 2),
      (3, 2, 3),
      (4, 1, 4),
      (2, 3, 2),
      (1, 5, 1),
    ];
    for (final (parent, child, expected) in cases) {
      final key = UniqueKey();
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: MergeSemantics(
            key: key,
            child: Semantics(
              headingLevel: parent,
              label: 'parent',
              child: Padding(
                padding: const EdgeInsets.all(1),
                child: Semantics(
                  headingLevel: child,
                  label: 'child',
                  child: const SizedBox(width: 40, height: 20),
                ),
              ),
            ),
          ),
        ),
      );
      final data = tester.getSemantics(find.byKey(key)).getSemanticsData();
      expect(data.label, 'parent\nchild');
      expect(
        data.headingLevel,
        expected,
        reason: 'parent=$parent child=$child',
      );
    }
  });

  testWidgets('singular FittedBox discards explicitly labeled scroll subtree', (
    tester,
  ) async {
    for (final (height, expected) in [(55.0, 1), (0.0, 0)]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Column(
            children: [
              SizedBox(
                width: 500,
                height: height,
                child: FittedBox(
                  child: SizedBox(
                    height: 55,
                    width: 266,
                    child: Semantics(
                      label: 'inside',
                      child: SingleChildScrollView(child: Column()),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      var matches = 0;
      void count(SemanticsNode node) {
        if (node.getSemanticsData().label == 'inside') matches++;
        node.visitChildren((child) {
          count(child);
          return true;
        });
      }

      count(RendererBinding.instance.renderView.debugSemantics!);
      expect(matches, expected);
    }
  });
}
