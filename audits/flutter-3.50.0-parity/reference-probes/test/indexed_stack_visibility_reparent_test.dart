import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'globally keyed branch keeps a stale value until its builder rebuilds',
    (tester) async {
      final key = GlobalKey();
      Widget makeLeaf() => Builder(
        key: key,
        builder: (context) => Text('${Visibility.of(context)}'),
      );
      final leaf = makeLeaf();
      Future<void> pump(bool inStack, int? selected, {bool refresh = false}) =>
          tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: Column(
                children: [
                  IndexedStack(
                    index: selected,
                    children: inStack
                        ? [refresh ? makeLeaf() : leaf]
                        : const [],
                  ),
                  Center(child: inStack ? null : leaf),
                ],
              ),
            ),
          );

      await pump(true, null);
      final retained = tester.element(find.byKey(key, skipOffstage: false));
      expect(find.text('false', skipOffstage: false), findsOneWidget);
      await pump(false, null);
      expect(tester.element(find.byKey(key)), same(retained));
      expect(find.text('true'), findsOneWidget);
      await pump(true, 0);
      expect(tester.element(find.byKey(key)), same(retained));
      expect(find.text('true'), findsOneWidget);
      await pump(true, null);
      expect(
        tester.element(find.byKey(key, skipOffstage: false)),
        same(retained),
      );
      // The retained Builder last ran outside the stack, where there was
      // no Visibility dependency to carry into the newly selected branch.
      // Its paint is now excluded, but the queried text remains stale.
      expect(find.text('true'), findsNothing);
      expect(find.text('true', skipOffstage: false), findsOneWidget);
      await pump(true, null, refresh: true);
      expect(
        tester.element(find.byKey(key, skipOffstage: false)),
        same(retained),
      );
      expect(find.text('false', skipOffstage: false), findsOneWidget);
    },
  );
}
