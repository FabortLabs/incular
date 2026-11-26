import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('primitive outline hit matrix has stable truth values', () {
    const rect = Rect.fromLTWH(0, 0, 120, 80);
    const points = <Offset>[
      Offset(-10, 40), Offset(1, 1), Offset(0.5, 14.5), Offset(1, 35),
      Offset(1, 68), Offset(11, 35), Offset(14.5, 13.5), Offset(21, 35),
    ];
    const rounded = RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(18)));
    const superellipse = RoundedSuperellipseBorder(borderRadius: BorderRadius.all(Radius.circular(18)));
    const circle = CircleBorder();
    const stadium = StadiumBorder();
    final shapes = <ShapeBorder>[
      Border.all(),
      circle,
      const CircleBorder(eccentricity: 0.5),
      const OvalBorder(),
      const RoundedRectangleBorder(),
      rounded,
      const RoundedRectangleBorder(
        borderRadius: BorderRadiusDirectional.only(topStart: Radius.circular(18)),
      ),
      const RoundedSuperellipseBorder(),
      superellipse,
      stadium,
      ShapeBorder.lerp(circle, rounded, 0.5)!,
      ShapeBorder.lerp(circle, superellipse, 0.5)!,
      ShapeBorder.lerp(stadium, circle, 0.5)!,
      ShapeBorder.lerp(stadium, rounded, 0.5)!,
    ];
    final matrix = [
      for (final border in shapes)
        [for (final point in points) if (border.hitTest(rect, point, textDirection: TextDirection.ltr)) '1' else '0'].join(),
    ];
    // Captured from the pinned reference; the Rust test consumes the same
    // truth table independently of its hit-test and Path implementations.
    expect(matrix, const <String>[
      '01111111',
      '00000001',
      '00000101',
      '00010101',
      '01111111',
      '00110111',
      '00111111',
      '01111111',
      '00010111',
      '00010111',
      '00000111',
      '00000101',
      '00000101',
      '00010111',
    ]);
  });

  test('stadium to rounded rectangle produces a distinct midpoint', () {
    const stadium = StadiumBorder();
    const rounded = RoundedRectangleBorder(
      borderRadius: BorderRadius.all(Radius.circular(18)),
    );
    const rect = Rect.fromLTWH(0, 0, 120, 80);
    final midpoint = ShapeBorder.lerp(stadium, rounded, 0.5)!;
    expect(stadium.hitTest(rect, const Offset(12, 8)), isFalse);
    expect(midpoint.hitTest(rect, const Offset(12, 8)), isTrue);
    expect(midpoint.hitTest(rect, const Offset(10, 5)), isFalse);
    expect(rounded.hitTest(rect, const Offset(10, 5)), isTrue);
  });
}
