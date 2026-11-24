import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Interrupted outgoing child is gone after 166ms', (tester) async {
    Widget build(int child) {
      return AnimatedSwitcher(
        duration: const Duration(milliseconds: 100),
        child: SizedBox.square(dimension: 50, key: ValueKey(child)),
      );
    }

    await tester.pumpWidget(build(1));
    await tester.pumpWidget(build(2));
    await tester.pump(const Duration(milliseconds: 50));
    await tester.pumpWidget(build(3));
    await tester.pump(const Duration(milliseconds: 10));
    await tester.pump(const Duration(milliseconds: 140));
    await tester.pump(const Duration(milliseconds: 16));
    expect(find.byKey(const ValueKey(1)), findsNothing);
    expect(find.byKey(const ValueKey(2)), findsNothing);
    expect(find.byKey(const ValueKey(3)), findsOneWidget);
  });
}
