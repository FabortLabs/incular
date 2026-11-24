import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget switcher(int key, {int reverseMs = 200, Curve curve = Curves.linear}) {
  return AnimatedSwitcher(
    duration: const Duration(milliseconds: 200),
    reverseDuration: Duration(milliseconds: reverseMs),
    switchInCurve: curve,
    child: SizedBox(key: ValueKey(key), width: 50, height: 50),
  );
}

void main() {
  testWidgets('outgoing duration belongs to the outgoing entry', (
    tester,
  ) async {
    for (final reverseMs in [200, 800]) {
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpWidget(switcher(1));
      await tester.pumpWidget(switcher(2, reverseMs: reverseMs));
      await tester.pump(const Duration(milliseconds: 100));
      final transition = find.ancestor(
        of: find.byKey(const ValueKey(1)),
        matching: find.byType(FadeTransition),
      );
      expect(tester.widget<FadeTransition>(transition).opacity.value, 0.5);
      await tester.pump(const Duration(milliseconds: 250));
      await tester.pump();
      expect(
        find.byKey(const ValueKey(1)),
        findsNothing,
        reason: 'incoming $reverseMs',
      );
      expect(find.byKey(const ValueKey(2)), findsOneWidget);
    }
  });

  testWidgets('changing the curve does not reshape an ongoing entry', (
    tester,
  ) async {
    double incomingOpacity() {
      final transition = find.ancestor(
        of: find.byKey(const ValueKey(2)),
        matching: find.byType(FadeTransition),
      );
      return tester.widget<FadeTransition>(transition).opacity.value;
    }

    await tester.pumpWidget(switcher(1));
    await tester.pumpWidget(switcher(2));
    await tester.pump(const Duration(milliseconds: 100));
    expect(incomingOpacity(), 0.5);
    await tester.pumpWidget(switcher(2));
    expect(incomingOpacity(), 0.5);
    await tester.pumpWidget(switcher(2, curve: Curves.easeIn));
    expect(incomingOpacity(), 0.5);
  });
}
