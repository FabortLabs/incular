import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('changing only the curve re-evaluates an active tween without restarting it', (tester) async {
    var latest = -1;
    var endings = 0;
    Widget view(Curve curve) => TweenAnimationBuilder<int>(
      tween: IntTween(begin: 0, end: 100),
      curve: curve,
      duration: const Duration(seconds: 1),
      onEnd: () => endings++,
      builder: (_, value, _) {
        latest = value;
        return const SizedBox();
      },
    );
    await tester.pumpWidget(view(Curves.linear));
    expect(latest, 0);
    await tester.pump(const Duration(milliseconds: 250));
    expect(latest, 25);
    await tester.pumpWidget(view(Curves.easeInQuad));
    expect(latest, (100 * Curves.easeInQuad.transform(0.25)).round());
    await tester.pump(const Duration(milliseconds: 500));
    expect(latest, (100 * Curves.easeInQuad.transform(0.75)).round());
    expect(endings, 0);
    // Flutter completes strictly after its duration, not at the exact endpoint.
    await tester.pump(const Duration(milliseconds: 251));
    expect(latest, 100);
    expect(endings, 1);
  });
}
