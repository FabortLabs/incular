import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('A reused PageView description does not share its position', (
    WidgetTester tester,
  ) async {
    final PageView shared = PageView(
      children: const [Text('zero'), Text('one'), Text('two')],
    );
    const Key first = ValueKey<String>('first');
    const Key second = ValueKey<String>('second');
    Widget inDirection(Widget child) =>
        Directionality(textDirection: TextDirection.ltr, child: child);
    await tester.pumpWidget(
      inDirection(
        Row(
          children: [
            Expanded(
              child: KeyedSubtree(key: first, child: shared),
            ),
            Expanded(
              child: KeyedSubtree(key: second, child: shared),
            ),
          ],
        ),
      ),
    );

    Finder page(Key key, String label) =>
        find.descendant(of: find.byKey(key), matching: find.text(label));
    expect(page(first, 'zero'), findsOneWidget);
    expect(page(second, 'zero'), findsOneWidget);
    await tester.timedDrag(
      find.byKey(first),
      const Offset(-265, 0),
      const Duration(milliseconds: 500),
    );
    await tester.pumpAndSettle();
    expect(page(first, 'one'), findsOneWidget);
    expect(page(second, 'zero'), findsOneWidget);
    expect(page(second, 'one'), findsNothing);

    await tester.pumpWidget(inDirection(const SizedBox()));
    await tester.pumpWidget(inDirection(shared));
    expect(find.text('zero'), findsOneWidget);
    expect(find.text('one'), findsNothing);
  });
}
