import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Blocked label merging preserves explicit child boundaries on update',
    (tester) async {
      const key = ValueKey('scope');
      const boundary = ValueKey('boundary');
      Widget build(String label, bool explicit) => Directionality(
        textDirection: TextDirection.ltr,
        child: Semantics(
          key: key,
          label: 'blocked',
          accessibilityFocusBlockType: AccessibilityFocusBlockType.blockSubtree,
          child: Column(
            children: [
              Semantics(
                container: explicit,
                label: label,
                child: const SizedBox.square(dimension: 10),
              ),
              Semantics(
                key: boundary,
                container: true,
                label: 'boundary',
                child: const SizedBox.square(dimension: 10),
              ),
              Semantics(
                label: 'last',
                child: const SizedBox.square(dimension: 10),
              ),
            ],
          ),
        ),
      );
      await tester.pumpWidget(build('first', false));
      final nodeId = tester.getSemantics(find.byKey(key)).id;
      final boundaryId = tester.getSemantics(find.byKey(boundary)).id;
      for (final (label, explicit, merged) in [
        ('first', false, 'blocked\nfirst\nlast'),
        ('updated', false, 'blocked\nupdated\nlast'),
        ('updated', true, 'blocked\nlast'),
        ('again', false, 'blocked\nagain\nlast'),
      ]) {
        await tester.pumpWidget(build(label, explicit));
        final scope = tester.getSemantics(find.byKey(key));
        final child = tester.getSemantics(find.byKey(boundary));
        expect(scope.id, nodeId);
        expect(child.id, boundaryId);
        expect(scope.label, merged);
        expect(
          child.getSemanticsData().flagsCollection.isAccessibilityFocusBlocked,
          isTrue,
        );
        final labels = <String>[];
        scope.visitChildren((node) {
          labels.add(node.label);
          return true;
        });
        expect(labels, explicit ? [label, 'boundary'] : ['boundary']);
      }
    },
  );
}
