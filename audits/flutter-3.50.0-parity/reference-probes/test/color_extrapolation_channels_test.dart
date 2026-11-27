import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('short hue arc extrapolates independently from clamped channels', () {
    const a = HSVColor.fromAHSV(0.75, 300, 0.25, 0.25);
    const b = HSVColor.fromAHSV(0.25, 60, 0.75, 0.75);
    expect(HSVColor.lerp(a, b, 1.5), const HSVColor.fromAHSV(0, 120, 1, 1));
    expect(HSVColor.lerp(a, b, -0.5), const HSVColor.fromAHSV(1, 240, 0, 0));

    const c = HSLColor.fromAHSL(0.75, 300, 0.25, 0.25);
    const d = HSLColor.fromAHSL(0.25, 60, 0.75, 0.75);
    expect(HSLColor.lerp(c, d, 1.5), const HSLColor.fromAHSL(0, 120, 1, 1));
    expect(HSLColor.lerp(c, d, -0.5), const HSLColor.fromAHSL(1, 240, 0, 0));
  });

  test('the 350 to 10 degree midpoint paints red, not cyan', () {
    expect(
      HSVColor.lerp(
        const HSVColor.fromAHSV(1, 350, 1, 1),
        const HSVColor.fromAHSV(1, 10, 1, 1),
        0.5,
      )!.toColor().toARGB32(),
      0xFFFF0000,
    );
    expect(
      HSLColor.lerp(
        const HSLColor.fromAHSL(1, 350, 1, 0.5),
        const HSLColor.fromAHSL(1, 10, 1, 0.5),
        0.5,
      )!.toColor().toARGB32(),
      0xFFFF0000,
    );
  });
}
