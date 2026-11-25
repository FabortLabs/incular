import 'package:flutter/animation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('animation style rounds durations and preserves absent endpoints', () {
    const first = AnimationStyle(
      duration: Duration(microseconds: 1),
      reverseDuration: Duration(microseconds: 3),
    );
    const second = AnimationStyle(duration: Duration(microseconds: 4));
    final middle = AnimationStyle.lerp(first, second, 0.5)!;
    expect(middle.duration, const Duration(microseconds: 3));
    expect(middle.reverseDuration, const Duration(microseconds: 2));
    expect(AnimationStyle.lerp(null, second, 0), const AnimationStyle());
    expect(AnimationStyle.lerp(first, null, 1), const AnimationStyle());
    expect(AnimationStyle.lerp(first, second, 1), second);
    // Dart durations can be signed; the Incular probe saturates at zero.
    expect(
      AnimationStyle.lerp(first, null, 1.5)!.duration,
      const Duration(microseconds: -1),
    );
  });

  test(
    'styles blend nested curve outputs and fall back to linear when absent',
    () {
      const first = AnimationStyle(
        curve: Curves.ease,
        reverseCurve: Curves.easeIn,
      );
      final mid = AnimationStyle.lerp(first, null, 0.25)!;
      final nested = AnimationStyle.lerp(
        mid,
        const AnimationStyle(curve: Curves.bounceIn),
        0.75,
      )!;
      for (final t in [0.0, 0.2, 0.5, 0.8, 1.0]) {
        final inner = Curves.ease.transform(t) * 0.75 + t * 0.25;
        final reversed = Curves.easeIn.transform(t) * 0.75 + t * 0.25;
        expect(
          nested.curve!.transform(t),
          inner * 0.25 + Curves.bounceIn.transform(t) * 0.75,
        );
        expect(nested.reverseCurve!.transform(t), reversed * 0.25 + t * 0.75);
      }
      expect(AnimationStyle.lerp(first, null, 1)!.curve, isNull);
      expect(AnimationStyle.lerp(first, null, 1)!.reverseCurve, isNull);
    },
  );
}
