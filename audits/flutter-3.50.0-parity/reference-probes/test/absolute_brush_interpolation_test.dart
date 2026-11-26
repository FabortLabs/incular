import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const black = Color(0xFF000000);
const white = Color(0xFFFFFFFF);

void main() {
  test(
    'interpolated gradients combine geometry and resample the stop union',
    () {
      const a = LinearGradient(
        colors: [black, white],
        tileMode: TileMode.clamp,
      );
      const b = LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [black, black, white],
        stops: [0, 0.5, 1],
        tileMode: TileMode.mirror,
      );
      final value = LinearGradient.lerp(a, b, 0.25)!;
      expect(value.begin, const Alignment(-0.75, -0.25));
      expect(value.end, const Alignment(0.75, 0.25));
      expect(value.stops, [0, 0.5, 1]);
      expect(value.colors[1].toARGB32(), 0xFF606060);
      expect(value.tileMode, TileMode.clamp);
      expect(LinearGradient.lerp(a, b, 0.5)!.tileMode, TileMode.mirror);
    },
  );

  test(
    'radial and sweep interpolation keep their class and interpolate endpoints',
    () {
      const a = RadialGradient(
        center: Alignment.topLeft,
        radius: 0.25,
        focal: Alignment.bottomLeft,
        focalRadius: 0.0625,
        colors: [black, white],
      );
      const b = RadialGradient(
        center: Alignment.bottomRight,
        radius: 0.75,
        focal: Alignment.topRight,
        focalRadius: 0.1875,
        colors: [white, black],
      );
      final radial = RadialGradient.lerp(a, b, 0.25)!;
      expect(radial.center, const Alignment(-0.5, -0.5));
      expect(radial.radius, 0.375);
      expect(radial.focal, const Alignment(-0.5, 0.5));
      expect(radial.focalRadius, 0.09375);

      const c = SweepGradient(
        startAngle: 0,
        endAngle: math.pi,
        colors: [black, white],
      );
      const d = SweepGradient(
        center: Alignment.bottomRight,
        startAngle: math.pi / 2,
        endAngle: math.pi * 2,
        colors: [white, black],
      );
      final sweep = SweepGradient.lerp(c, d, 0.25)!;
      expect(sweep.center, const Alignment(0.25, 0.25));
      expect(sweep.startAngle, closeTo(math.pi / 8, 0.0001));
      expect(sweep.endAngle, closeTo(math.pi * 1.25, 0.0001));
    },
  );

  test('incompatible gradient classes fade to transparent at their switch', () {
    const a = LinearGradient(colors: [black, white]);
    const b = RadialGradient(colors: [white, black]);
    final before = Gradient.lerp(a, b, 0.25)!;
    final at = Gradient.lerp(a, b, 0.5)!;
    final after = Gradient.lerp(a, b, 0.75)!;
    expect(before, isA<LinearGradient>());
    expect(before.colors[0].a, 0.5);
    expect(at, isA<RadialGradient>());
    expect(at.colors.every((color) => color.a == 0), isTrue);
    expect(after, isA<RadialGradient>());
    expect(after.colors[0].a, 0.5);
  });
}
