import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool replace in <bool>[false, true]) {
    testWidgets(
      'reversal ignores a ${replace ? 'replaced' : 'removed'} predecessor',
      (WidgetTester tester) async {
        final FocusScopeNode scope = FocusScopeNode();
        final FocusNode first = FocusNode();
        final FocusNode aligned = FocusNode();
        final FocusNode current = FocusNode();
        final FocusNode replacement = FocusNode();
        final GlobalKey firstKey = GlobalKey();
        for (final FocusNode node in <FocusNode>[
          scope,
          first,
          aligned,
          current,
          replacement,
        ]) {
          addTearDown(node.dispose);
        }

        Widget stop(FocusNode node) => Focus(
          focusNode: node,
          child: const SizedBox(width: 100, height: 100),
        );

        Future<void> show({required bool updated}) => tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: FocusScope(
              node: scope,
              child: Stack(
                children: <Widget>[
                  Positioned(
                    key: firstKey,
                    top: 0,
                    left: 0,
                    child: !updated
                        ? stop(first)
                        : replace
                        ? stop(replacement)
                        : const SizedBox(width: 100, height: 100),
                  ),
                  Positioned(top: 0, left: 300, child: stop(aligned)),
                  Positioned(top: 100, left: 300, child: stop(current)),
                ],
              ),
            ),
          ),
        );

        await show(updated: false);
        first.requestFocus();
        await tester.pump();
        expect(first.hasPrimaryFocus, isTrue);
        expect(scope.focusInDirection(TraversalDirection.down), isTrue);
        await tester.pump();
        expect(current.hasPrimaryFocus, isTrue);
        await show(updated: true);
        expect(current.hasPrimaryFocus, isTrue);
        expect(scope.focusInDirection(TraversalDirection.up), isTrue);
        await tester.pump();
        expect(aligned.hasPrimaryFocus, isTrue);
        expect(first.hasFocus || replacement.hasFocus, isFalse);
      },
    );
  }
}
