import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('elliptical superellipse contour retains axes before clamping', () {
    const area = Rect.fromLTWH(0, 0, 120, 300);
    const radii = <BorderRadius>[
      BorderRadius.all(Radius.elliptical(20, 50)),
      BorderRadius.all(Radius.circular(20)),
      BorderRadius.all(Radius.circular(600)),
      BorderRadius.only(
        topLeft: Radius.elliptical(1000, 1000),
        topRight: Radius.elliptical(0, 1000),
        bottomRight: Radius.elliptical(800, 1000),
        bottomLeft: Radius.elliptical(100, 500),
      ),
    ];
    const samples = <Offset>[
      Offset(1, 1),
      Offset(4, 5),
      Offset(4, 20),
      Offset(4, 35),
      Offset(5, 2),
      Offset(10, 2),
      Offset(22, 5),
      Offset(35, 4),
      Offset(60, 150),
      Offset(116, 2),
      Offset(4, 298),
      Offset(118, 298),
      Offset(50, 20),
      Offset(60, 5),
    ];
    const included = <List<int>>[
      [3, 6, 7, 8, 12, 13],
      [2, 3, 6, 7, 8, 12, 13],
      [8, 12, 13],
      [8, 9],
    ];
    for (int i = 0; i < radii.length; i++) {
      final border = RoundedSuperellipseBorder(borderRadius: radii[i]);
      final path = border.getOuterPath(area);
      expect(path.getBounds(), area);
      expect([
        for (int j = 0; j < samples.length; j++)
          if (path.contains(samples[j])) j,
      ], included[i]);
    }
  });
}
