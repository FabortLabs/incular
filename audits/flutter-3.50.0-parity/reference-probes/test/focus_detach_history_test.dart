import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final requestReplacement in [false, true]) {
    testWidgets(
      'detach restores history unless replacement has a request ($requestReplacement)',
      (tester) async {
        final scope = FocusScopeNode();
        final earlier = FocusNode();
        final old = FocusNode();
        final replacement = FocusNode();
        try {
          Widget scene(FocusNode last) => Directionality(
            textDirection: TextDirection.ltr,
            child: FocusScope(
              node: scope,
              child: Column(
                children: [
                  Focus(focusNode: earlier, child: const SizedBox.shrink()),
                  Focus(focusNode: last, child: const SizedBox.shrink()),
                ],
              ),
            ),
          );

          await tester.pumpWidget(scene(old));
          earlier.requestFocus();
          await tester.pump();
          expect(earlier.hasPrimaryFocus, isTrue);
          old.requestFocus();
          await tester.pump();
          expect(old.hasPrimaryFocus, isTrue);
          if (requestReplacement) {
            replacement.requestFocus();
          }

          await tester.pumpWidget(scene(replacement));
          await tester.pump();
          expect(old.hasPrimaryFocus, isFalse);
          expect(replacement.hasPrimaryFocus, requestReplacement);
          expect(earlier.hasPrimaryFocus, !requestReplacement);
          expect(
            scope.focusedChild,
            requestReplacement ? replacement : earlier,
          );
        } finally {
          await tester.pumpWidget(const SizedBox.shrink());
          for (final node in [earlier, old, replacement, scope]) {
            node.dispose();
          }
        }
      },
    );
  }

  testWidgets('detaching the first child does not replay surviving autofocus', (
    tester,
  ) async {
    final scope = FocusScopeNode();
    final first = FocusNode();
    final survivor = FocusNode();
    try {
      Widget scene(bool includeFirst) => Directionality(
        textDirection: TextDirection.ltr,
        child: FocusScope(
          node: scope,
          child: Column(
            children: [
              if (includeFirst)
                Focus(
                  key: const ValueKey('first'),
                  focusNode: first,
                  autofocus: true,
                  child: const SizedBox.shrink(),
                ),
              Focus(
                key: const ValueKey('survivor'),
                focusNode: survivor,
                autofocus: true,
                child: const SizedBox.shrink(),
              ),
            ],
          ),
        ),
      );
      await tester.pumpWidget(scene(true));
      expect(first.hasPrimaryFocus, isTrue);
      expect(survivor.hasPrimaryFocus, isFalse);
      final element = tester.element(find.byKey(const ValueKey('survivor')));
      await tester.pumpWidget(scene(false));
      await tester.pump();
      expect(
        tester.element(find.byKey(const ValueKey('survivor'))),
        same(element),
      );
      expect(survivor.hasPrimaryFocus, isFalse);
      expect(scope.hasPrimaryFocus, isTrue);
    } finally {
      await tester.pumpWidget(const SizedBox.shrink());
      first.dispose();
      survivor.dispose();
      scope.dispose();
    }
  });
}
