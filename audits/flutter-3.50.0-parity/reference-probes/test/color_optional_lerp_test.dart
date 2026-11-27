import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('optional HSL and HSV transitions preserve hue while alpha scales', () {
    const hsl = HSLColor.fromAHSL(0.5, 360, 0.4, 0.6);
    const hsv = HSVColor.fromAHSV(0.5, 360, 0.4, 0.6);
    expect(identical(HSLColor.lerp(hsl, hsl, 0.5), hsl), isTrue);
    expect(identical(HSVColor.lerp(hsv, hsv, 0.5), hsv), isTrue);
    expect(
      HSLColor.lerp(null, hsl, 0.25),
      const HSLColor.fromAHSL(0.125, 360, 0.4, 0.6),
    );
    expect(
      HSVColor.lerp(null, hsv, 0.25),
      const HSVColor.fromAHSV(0.125, 360, 0.4, 0.6),
    );
    expect(
      HSLColor.lerp(hsl, null, 0.25),
      const HSLColor.fromAHSL(0.375, 360, 0.4, 0.6),
    );
    expect(
      HSVColor.lerp(hsv, null, 0.25),
      const HSVColor.fromAHSV(0.375, 360, 0.4, 0.6),
    );
    // An extrapolated factor can still produce a valid alpha.
    expect(
      HSLColor.lerp(null, hsl, 1.5),
      const HSLColor.fromAHSL(0.75, 360, 0.4, 0.6),
    );
    expect(
      HSVColor.lerp(hsv, null, -0.5),
      const HSVColor.fromAHSV(0.75, 360, 0.4, 0.6),
    );
  });
}
