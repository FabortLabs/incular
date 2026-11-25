import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('only the selected merged child owns a custom action', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      const a = CustomSemanticsAction(label: 'first action');
      const b = CustomSemanticsAction(label: 'second action');
      final ids = [
        CustomSemanticsAction.getIdentifier(a),
        CustomSemanticsAction.getIdentifier(b),
      ];
      final seen = [0, 0];

      Widget child(String label, CustomSemanticsAction action, int index) {
        return Semantics(
          label: label,
          customSemanticsActions: {action: () => seen[index]++},
          child: const SizedBox(width: 10, height: 10),
        );
      }

      Widget tree(int index) => Directionality(
        textDirection: TextDirection.ltr,
        child: MergeSemantics(
          child: IndexedStack(
            index: index,
            children: [child('First', a, 0), child('Second', b, 1)],
          ),
        ),
      );

      int? oldId;
      for (final active in [0, 1]) {
        await tester.pumpWidget(tree(active));
        final node = tester.getSemantics(find.byType(MergeSemantics));
        expect(node.getSemanticsData().customSemanticsActionIds, [ids[active]]);
        if (oldId != null) {
          expect(node.id, oldId);
        }
        oldId = node.id;
        final owner = tester.binding.pipelineOwner.semanticsOwner!;
        owner.performAction(
          node.id,
          SemanticsAction.customAction,
          ids[1 - active],
        );
        owner.performAction(node.id, SemanticsAction.customAction, ids[active]);
        expect(seen, [1, active]);
      }
    } finally {
      semantics.dispose();
    }
  });
}
