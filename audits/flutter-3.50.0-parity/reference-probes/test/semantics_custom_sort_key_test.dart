import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Descending extends OrdinalSortKey {
  const _Descending(super.order, {super.name});
  @override
  int doCompare(OrdinalSortKey other) => -super.doCompare(other);
}

class _Ascending extends OrdinalSortKey {
  const _Ascending(super.order, {super.name});
}

void main() {
  testWidgets(
    'type and name delimit custom sort runs, and equal keys stay in paint order',
    (tester) async {
      Widget node(SemanticsSortKey key, String label) =>
          Semantics(sortKey: key, child: Text(label));

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Row(
            children: [
              node(const _Descending(1, name: 'group'), 'a1'),
              node(const _Descending(3, name: 'group'), 'a3'),
              node(const _Descending(3, name: 'group'), 'a3 tie'),
              node(const OrdinalSortKey(2, name: 'group'), 'b2'),
              node(const OrdinalSortKey(0, name: 'group'), 'b0'),
              node(const _Ascending(9, name: 'group'), 'c9'),
              node(const _Ascending(1, name: 'group'), 'c1'),
              node(const _Descending(2, name: 'group'), 'd2'),
              node(const _Descending(4, name: 'other'), 'e4'),
              node(const _Descending(6, name: 'other'), 'e6'),
            ],
          ),
        ),
      );
      final SemanticsNode root =
          tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
      final children = root.debugListChildrenInOrder(
        DebugSemanticsDumpOrder.traversalOrder,
      );
      expect(children.map((node) => node.label), [
        'a3',
        'a3 tie',
        'a1',
        'b0',
        'b2',
        'c1',
        'c9',
        'd2',
        'e6',
        'e4',
      ]);
    },
  );
}
