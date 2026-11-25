import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('maintained bottom uses persistent padding instead of maximum', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MediaQuery(
        data: MediaQueryData(
          padding: EdgeInsets.only(bottom: 35),
          viewPadding: EdgeInsets.only(bottom: 20),
        ),
        child: SafeArea(maintainBottomViewPadding: true, child: Placeholder()),
      ),
    );
    expect(tester.getSize(find.byType(Placeholder)), const Size(800, 580));
  });

  testWidgets('nested maintained areas share unconsumed view padding only', (
    tester,
  ) async {
    const child = SafeArea(
      maintainBottomViewPadding: true,
      child: SafeArea(maintainBottomViewPadding: true, child: Placeholder()),
    );
    Future<void> pump(double safe) => tester.pumpWidget(
      MediaQuery(
        data: MediaQueryData(
          padding: EdgeInsets.only(bottom: safe),
          viewPadding: const EdgeInsets.only(bottom: 20),
        ),
        child: child,
      ),
    );
    // While safe padding includes the persistent margin, the outer
    // SafeArea removes it before the inner widget reads it.
    await pump(20);
    expect(tester.getSize(find.byType(Placeholder)), const Size(800, 580));
    final identity = tester.element(find.byType(Placeholder));
    // When a keyboard has consumed safe padding, subtracting that zero
    // leaves the persistent margin visible to both maintained areas.
    await pump(0);
    expect(tester.getSize(find.byType(Placeholder)), const Size(800, 560));
    expect(tester.element(find.byType(Placeholder)), same(identity));
  });
}
