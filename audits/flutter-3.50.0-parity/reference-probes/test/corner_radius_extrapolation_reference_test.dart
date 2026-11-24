import 'dart:ui';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Radius and BorderRadius retain overshooting negative components', () {
    const a = Radius.elliptical(10, 2);
    const b = Radius.elliptical(2, 10);
    expect(Radius.lerp(a, b, .5), const Radius.elliptical(6, 6));
    const extrapolated = Radius.elliptical(-2, 14);
    expect(Radius.lerp(a, b, 1.5), extrapolated);
    expect(
      BorderRadius.lerp(
        const BorderRadius.all(a),
        const BorderRadius.all(b),
        1.5,
      ),
      const BorderRadius.all(extrapolated),
    );
  });
}
