import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('HSV and HSL choose the short arc across 360', () {
    final hsv = HSVColor.lerp(
      const HSVColor.fromAHSV(1, 350, 1, 1),
      const HSVColor.fromAHSV(1, 10, 1, 1),
      0.5,
    )!;
    final hsl = HSLColor.lerp(
      const HSLColor.fromAHSL(1, 350, 1, .5),
      const HSLColor.fromAHSL(1, 10, 1, .5),
      .5,
    )!;
    expect([hsv.hue, hsl.hue], [0, 0]);
  });
  test('HSL interpolates hue past the endpoint', () {
    final hsl = HSLColor.lerp(
      const HSLColor.fromAHSL(1, 20, 1, .5),
      const HSLColor.fromAHSL(1, 60, 1, .5),
      1.5,
    )!;
    expect(hsl.hue, 80);
  });
}
