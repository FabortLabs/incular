import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void assertOrder(WidgetTester tester, String label, List<String> expected) {
  final group = tester.semantics.find(find.bySemanticsLabel(label));
  final dump = group.toStringDeep(
    childOrder: DebugSemanticsDumpOrder.traversalOrder,
  );
  final positions = expected
      .map((name) => dump.indexOf('label: "$name"'))
      .toList();
  expect(positions, everyElement(greaterThanOrEqualTo(0)), reason: dump);
  expect(positions, orderedEquals([...positions]..sort()), reason: dump);
}

void main() {
  for (final direction in TextDirection.values) {
    testWidgets(
      'semantic traversal follows separated and overlapping geometry: $direction',
      (tester) async {
        final handle = tester.ensureSemantics();
        try {
          Widget cell(String label, double left, double top) => Positioned(
            left: left,
            top: top,
            child: Semantics(
              label: label,
              button: true,
              child: const SizedBox(width: 40, height: 20),
            ),
          );
          Widget scene(double right) => Directionality(
            textDirection: direction,
            child: Semantics(
              label: 'reading group',
              container: true,
              explicitChildNodes: true,
              child: Stack(
                children: [
                  cell('below', 60, 80),
                  cell('right', right, 5),
                  cell('left', 5, 5),
                ],
              ),
            ),
          );
          final expected = direction == TextDirection.ltr
              ? ['left', 'right', 'below']
              : ['right', 'left', 'below'];
          for (final right in [130.0, 25.0]) {
            await tester.pumpWidget(scene(right));
            assertOrder(tester, 'reading group', expected);
          }
        } finally {
          handle.dispose();
        }
      },
    );
  }
  testWidgets('a visual move keeps the node and updates traversal order', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    try {
      const cell = SizedBox(width: 40, height: 20);
      Widget scene(double shift) => Directionality(
        textDirection: TextDirection.ltr,
        child: Semantics(
          label: 'moving group',
          container: true,
          explicitChildNodes: true,
          child: Stack(
            children: [
              Positioned(
                left: 70,
                top: 5,
                child: Semantics(label: 'stationary', child: cell),
              ),
              Positioned(
                left: 130,
                top: 5,
                child: Transform.translate(
                  offset: Offset(shift, 0),
                  child: Semantics(label: 'moving', child: cell),
                ),
              ),
            ],
          ),
        ),
      );
      await tester.pumpWidget(scene(0));
      assertOrder(tester, 'moving group', ['stationary', 'moving']);
      final id = tester.semantics.find(find.bySemanticsLabel('moving')).id;
      await tester.pumpWidget(scene(-120));
      assertOrder(tester, 'moving group', ['moving', 'stationary']);
      expect(tester.semantics.find(find.bySemanticsLabel('moving')).id, id);
    } finally {
      handle.dispose();
    }
  });
}
