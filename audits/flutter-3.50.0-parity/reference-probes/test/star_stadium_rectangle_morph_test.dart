import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

const rect = Rect.fromLTWH(0, 0, 120, 80);
const star = StarBorder(
  points: 5.5,
  innerRadiusRatio: 0.35,
  rotation: 15,
  side: BorderSide(width: 6),
);

String sample(Path path) => [
  for (final y in [5.0, 20.0, 40.0, 60.0, 75.0])
    for (final x in [10.0, 25.0, 45.0, 60.0, 75.0, 95.0, 110.0])
      if (path.contains(Offset(x, y))) '1' else '0',
].join();

void main() {
  const stadium = StadiumBorder(side: BorderSide(width: 2));
  const rectangle = RoundedRectangleBorder(
    side: BorderSide(width: 2),
    borderRadius: BorderRadius.only(
      topLeft: Radius.elliptical(18, 12),
      topRight: Radius.elliptical(12, 24),
      bottomRight: Radius.circular(4),
    ),
  );
  for (final (name, endpoint, progress, width) in [
    ('stadium to circle', stadium, 0.25, 2.5),
    ('circle to star', stadium, 0.75, 5.5),
    ('rectangle to stadium', rectangle, 1 / 6, 7 / 3),
    ('stadium to circle from rectangle', rectangle, 0.5, 4.0),
    ('circle to star from rectangle', rectangle, 5 / 6, 17 / 3),
  ]) {
    test(name, () {
      final a = ShapeBorder.lerp(endpoint, star, progress)!;
      final b = ShapeBorder.lerp(star, endpoint, 1 - progress)!;
      expect(
        a.dimensions.resolve(TextDirection.ltr).left,
        closeTo(width, 0.001),
      );
      expect(
        b.dimensions.resolve(TextDirection.ltr).left,
        closeTo(width, 0.001),
      );
      final outer = sample(a.getOuterPath(rect));
      final inner = sample(a.getInnerPath(rect));
      const expected = {
        'stadium to circle': (
          '00111000111110111111101111100011100',
          '00111000111110011111001111100011100',
        ),
        'circle to star': (
          '00001000011100011111000111000010000',
          '00000000011100001110000111000000000',
        ),
        'rectangle to stadium': (
          '01111101111111111111111111111111111',
          '01111101111111111111111111110111110',
        ),
        'stadium to circle from rectangle': (
          '00111000111110111111101111100011100',
          '00111000111110011111001111100011100',
        ),
        'circle to star from rectangle': (
          '00001000011100011111000111000010000',
          '00000000011100001110000111000000000',
        ),
      };
      expect((outer, inner), expected[name]);
      expect(sample(b.getOuterPath(rect)), outer);
      expect(sample(b.getInnerPath(rect)), inner);
    });
  }
}
