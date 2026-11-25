import 'dart:ui' show SemanticsAction;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'merged tap keeps an explicit callback under IgnorePointer',
    (tester) async {
      final handle = tester.ensureSemantics();
      final hits = <String>[];
      Widget target(String label, bool ignored) {
        final child = Semantics(
          label: label,
          onTap: () => hits.add(label),
          child: const SizedBox(width: 60, height: 40),
        );
        return ignored ? IgnorePointer(child: child) : child;
      }

      Widget scene(bool north, bool south, {String key = 'retained'}) =>
          Directionality(
            textDirection: TextDirection.ltr,
            child: MergeSemantics(
              key: ValueKey(key),
              child: Column(
                children: [target('North', north), target('South', south)],
              ),
            ),
          );

      await tester.pumpWidget(scene(false, false));
      final id = tester.getSemantics(find.byType(MergeSemantics)).id;
      for (final (north, south, expected) in [
        (false, false, 'North'),
        (true, false, 'North'),
      ]) {
        hits.clear();
        await tester.pumpWidget(scene(north, south));
        final node = tester.getSemantics(find.byType(MergeSemantics));
        expect(node.id, id);
        expect(
          node.getSemanticsData().hasAction(SemanticsAction.tap),
          expected != null,
        );
        tester.binding.pipelineOwner.semanticsOwner!.performAction(
          id,
          SemanticsAction.tap,
        );
        expect(hits, expected == null ? isEmpty : [expected]);
      }
      hits.clear();
      await tester.pumpWidget(scene(true, false, key: 'fresh'));
      final fresh = tester.getSemantics(find.byType(MergeSemantics)).id;
      expect(fresh, isNot(id));
      tester.binding.pipelineOwner.semanticsOwner!.performAction(
        fresh,
        SemanticsAction.tap,
      );
      expect(hits, ['North']);
      handle.dispose();
    },
  );
}
