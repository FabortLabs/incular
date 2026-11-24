import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('matching unbounded maxima interpolate other finite bounds', () {
    const a = BoxConstraints(minHeight: 10, maxHeight: 20);
    const b = BoxConstraints(minHeight: 20, maxHeight: 30);
    expect(
      BoxConstraints.lerp(a, b, .5),
      const BoxConstraints(minHeight: 15, maxHeight: 25),
    );
  });

  test('mixed finite and unbounded maxima reject even with finite minima', () {
    const finite = BoxConstraints(maxWidth: 100, minHeight: 10, maxHeight: 20);
    const unbounded = BoxConstraints(minHeight: 20, maxHeight: 30);
    expect(
      () => BoxConstraints.lerp(finite, unbounded, .5),
      throwsAssertionError,
    );
    expect(
      () => BoxConstraints.lerp(unbounded, finite, .5),
      throwsAssertionError,
    );
  });
}
