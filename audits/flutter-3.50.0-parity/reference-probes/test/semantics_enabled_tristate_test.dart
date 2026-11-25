import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('enabled metadata can be unset without disabling the action', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    try {
      const key = ValueKey('subject');
      for (final enabled in [null, true, false, null]) {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Semantics(
              key: key,
              container: true,
              label: 'Action',
              button: true,
              onTap: () {},
              enabled: enabled,
              child: const SizedBox(width: 120, height: 40),
            ),
          ),
        );
        expect(
          tester
              .getSemantics(find.byKey(key))
              .getSemanticsData()
              .flagsCollection
              .isEnabled
              .toBoolOrNull(),
          enabled,
        );
      }
    } finally {
      handle.dispose();
    }
  });

  testWidgets('a forced merge combines enabled tri-state flags', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    try {
      const key = ValueKey('merge');
      for (final (first, second, expected) in [
        (null, null, null),
        (null, false, false),
        (false, true, true),
        (true, false, true),
      ]) {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: MergeSemantics(
              key: key,
              child: Column(
                children: [
                  Semantics(
                    label: 'First',
                    enabled: first,
                    child: const SizedBox.square(dimension: 10),
                  ),
                  Semantics(
                    label: 'Second',
                    enabled: second,
                    child: const SizedBox.square(dimension: 10),
                  ),
                ],
              ),
            ),
          ),
        );
        final node = tester.getSemantics(find.byKey(key));
        expect(node.getSemanticsData().label, contains('First'));
        expect(node.getSemanticsData().label, contains('Second'));
        expect(
          node.getSemanticsData().flagsCollection.isEnabled.toBoolOrNull(),
          expected,
        );
      }
    } finally {
      handle.dispose();
    }
  });
}
