import 'dart:ui' show SemanticsAction;

import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'declared action conflict splits and remerges a retained annotations owner',
    (tester) async {
      const ownerKey = ValueKey<String>('owner');
      const firstKey = ValueKey<String>('first');
      const secondKey = ValueKey<String>('second');
      final trace = <String>[];
      Widget frame(bool conflicting) => Directionality(
        textDirection: TextDirection.ltr,
        child: Semantics(
          key: ownerKey,
          container: true,
          child: Column(
            children: [
              Semantics(
                key: firstKey,
                label: 'label1',
                blockUserActions: true,
                onTap: () => trace.add('first'),
                child: const SizedBox(width: 10, height: 10),
              ),
              Semantics(
                key: secondKey,
                label: 'label2',
                onTap: conflicting ? () => trace.add('second tap') : null,
                onLongPress: conflicting
                    ? null
                    : () => trace.add('second long'),
                child: const SizedBox(width: 10, height: 10),
              ),
            ],
          ),
        ),
      );

      await tester.pumpWidget(frame(false));
      final int stable = tester.getSemantics(find.byKey(ownerKey)).id;
      final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
      void expectMerged() {
        final SemanticsNode node = tester.getSemantics(find.byKey(ownerKey));
        expect(node.id, stable);
        expect(node.label, 'label1\nlabel2');
        expect(node.childrenCount, 0);
        expect(node.getSemanticsData().hasAction(SemanticsAction.tap), isFalse);
        expect(
          node.getSemanticsData().hasAction(SemanticsAction.longPress),
          isTrue,
        );
      }

      expectMerged();
      await tester.pumpWidget(frame(true));
      final SemanticsNode split = tester.getSemantics(find.byKey(ownerKey));
      final SemanticsNode first = tester.getSemantics(find.byKey(firstKey));
      final SemanticsNode second = tester.getSemantics(find.byKey(secondKey));
      expect(split.id, stable);
      expect(split.childrenCount, 2);
      expect(first.label, 'label1');
      expect(second.label, 'label2');
      expect(first.getSemanticsData().hasAction(SemanticsAction.tap), isFalse);
      expect(second.getSemanticsData().hasAction(SemanticsAction.tap), isTrue);
      owner.performAction(stable, SemanticsAction.longPress);
      owner.performAction(second.id, SemanticsAction.tap);
      expect(trace, ['second tap']);

      await tester.pumpWidget(frame(false));
      expectMerged();
      owner.performAction(second.id, SemanticsAction.tap);
      owner.performAction(stable, SemanticsAction.longPress);
      expect(trace, ['second tap', 'second long']);
    },
  );
}
