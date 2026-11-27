import 'dart:math' as math;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

const rect = Rect.fromLTWH(0, 0, 120, 80);
const circle = CircleBorder(eccentricity: 0.6, side: BorderSide(width: 2));

String sample(Path path) => [
  for (final y in [5.0, 20.0, 40.0, 60.0, 75.0])
    for (final x in [10.0, 25.0, 45.0, 60.0, 75.0, 95.0, 110.0])
      if (path.contains(Offset(x, y))) '1' else '0',
].join();

void main() {
  test('a fractional star morphs from an eccentric circle and back', () {
    const star = StarBorder(
      points: 5.5,
      rotation: 15,
      squash: 0.1,
      pointRounding: 0.2,
      valleyRounding: 0.1,
      innerRadiusRatio: 0.35,
      side: BorderSide(width: 6),
    );
    final first = ShapeBorder.lerp(circle, star, 0.25)! as StarBorder;
    final second = ShapeBorder.lerp(star, circle, 0.75)! as StarBorder;
    for (final current in [first, second]) {
      expect(current.side.width, closeTo(3, 0.00001));
      expect(current.points, closeTo(5.875, 0.00001));
      expect(current.rotation, closeTo(15, 0.00001));
      expect(current.squash, closeTo(0.475, 0.00001));
      expect(current.pointRounding, closeTo(0.8, 0.00001));
      expect(current.valleyRounding, closeTo(0.025, 0.00001));
      final seed = math.cos(math.pi / 5.875);
      expect(
        current.innerRadiusRatio,
        closeTo(seed + (0.35 - seed) * 0.25, 0.00001),
      );
    }
    final outer = sample(first.getOuterPath(rect));
    final inner = sample(first.getInnerPath(rect));
    expect(outer, '00011000111110011111001111100011000');
    expect(inner, '00010000111110011111000111100011000');
    expect(sample(second.getOuterPath(rect)), outer);
    expect(sample(second.getInnerPath(rect)), inner);
  });

  test('near two points both directions use their own progress toward two', () {
    const star = StarBorder(
      points: 2.2,
      pointRounding: 0.1,
      valleyRounding: 0.1,
      innerRadiusRatio: 0.4,
    );
    final from = ShapeBorder.lerp(circle, star, 0.25)! as StarBorder;
    final to = ShapeBorder.lerp(star, circle, 0.75)! as StarBorder;
    expect(from.points, closeTo(2.15, 0.00001));
    expect(to.points, closeTo(2.05, 0.00001));
    for (final border in [from, to]) {
      expect(border.pointRounding, closeTo(0.4, 0.00001));
      expect(border.valleyRounding, closeTo(0.4, 0.00001));
      expect(border.innerRadiusRatio, closeTo(0.85, 0.00001));
    }
  });
}
