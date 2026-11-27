import 'dart:ui';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Star outlines and their inset on a translated rectangle', () {
    const rect = Rect.fromLTWH(7, 11, 200, 100);
    const xs = <double>[
      1.25,
      19.75,
      41.25,
      63.75,
      84.25,
      101.75,
      117.25,
      137.75,
      155.25,
      177.75,
      195.25,
    ];
    const ys = <double>[2.25, 16.75, 30.25, 43.75, 58.25, 71.75, 86.25, 98.75];
    String sample(Path outline) => ys
        .map(
          (y) => xs
              .map(
                (x) =>
                    outline.contains(rect.topLeft + Offset(x, y)) ? '#' : '.',
              )
              .join(),
        )
        .join('/');
    for (final (name, border) in <(String, StarBorder)>[
      ('default', const StarBorder()),
      ('triangle', const StarBorder.polygon()),
      (
        'two-point-polygon',
        const StarBorder.polygon(sides: 2, pointRounding: 0.2, squash: 0.7),
      ),
      (
        'zero-valley',
        const StarBorder(
          innerRadiusRatio: 0,
          points: 6,
          rotation: 27,
          squash: 0.7,
        ),
      ),
      for (final (name, align) in [
        ('inside', -1.0),
        ('center', 0.0),
        ('outside', 1.0),
      ])
        (
          name,
          StarBorder(
            points: 5.2,
            innerRadiusRatio: 0.55,
            pointRounding: 0.35,
            valleyRounding: 0.4,
            rotation: 27,
            squash: 0.6,
            side: BorderSide(width: 9.5, strokeAlign: align),
          ),
        ),
    ]) {
      final outer = border.getOuterPath(rect);
      final inner = border.getInnerPath(rect);
      expect(outer.contains(rect.center), isTrue, reason: name);
      print([name, sample(outer), sample(inner)].join(' '));
    }
  });
}
