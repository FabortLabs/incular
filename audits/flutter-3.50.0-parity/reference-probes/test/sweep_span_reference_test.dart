import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

const _bounds = Rect.fromLTWH(0, 0, 64, 64);
const _red = Color(0xffff0000);
const _blue = Color(0xff0000ff);
const _sites = [(52, 32), (32, 52), (12, 32), (32, 12)];

Future<List<List<int>>> _pixels(SweepGradient gradient) async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  canvas.drawRect(_bounds, Paint()..shader = gradient.createShader(_bounds));
  final picture = recorder.endRecording();
  final image = await picture.toImage(64, 64);
  final data = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  final samples = [
    for (final (x, y) in _sites)
      [for (var i = 0; i < 4; i++) data.getUint8((y * 64 + x) * 4 + i)],
  ];
  image.dispose();
  picture.dispose();
  return samples;
}

void main() {
  testWidgets('partial sweep samples around its branch cut and rotation', (
    tester,
  ) async {
    await tester.runAsync(() async {
      const clamp = SweepGradient(
        colors: [_red, _blue],
        startAngle: math.pi / 4,
        endAngle: math.pi * 3 / 4,
      );
      const decal = SweepGradient(
        colors: [_red, _blue],
        startAngle: math.pi / 4,
        endAngle: math.pi * 3 / 4,
        tileMode: TileMode.decal,
      );
      const rotated = SweepGradient(
        colors: [_red, _blue],
        startAngle: math.pi / 4,
        endAngle: math.pi * 3 / 4,
        tileMode: TileMode.decal,
        transform: GradientRotation(math.pi / 2),
      );
      final c = await _pixels(clamp);
      final d = await _pixels(decal);
      final r = await _pixels(rotated);
      expect(c[0], [255, 0, 0, 255]);
      expect(c[2], [0, 0, 255, 255]);
      expect(c[3], [0, 0, 255, 255]);
      for (final pixel in [c[1], d[1], r[2]]) {
        expect(pixel[0], closeTo(131, 2));
        expect(pixel[1], 0);
        expect(pixel[2], closeTo(123, 2));
        expect(pixel[3], 255);
      }
      for (final pixel in [d[0], d[2], d[3], r[0], r[1], r[3]]) {
        expect(pixel, [0, 0, 0, 0]);
      }
    });
  });
}
