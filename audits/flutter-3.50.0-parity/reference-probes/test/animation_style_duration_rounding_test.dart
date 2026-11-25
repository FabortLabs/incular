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
}
