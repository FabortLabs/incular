import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';

const _surface = ui.Rect.fromLTWH(0, 0, 64, 64);
const _center = ui.Offset(32.5, 32.5);
const _red = ui.Color(0xffff0000);
const _blue = ui.Color(0xff0000ff);
const _transparentBlue = ui.Color(0x000000ff);
const _black = ui.Color(0xff000000);
const _stops = [0.0, 1.0];

enum _Geometry {
  linear([(20, 32), (32, 32), (44, 32)]),
  radial([(36, 32), (40, 32), (44, 32)]),
  sweep([(32, 48), (16, 32), (32, 16)]);

  const _Geometry(this.samples);
  final List<(int, int)> samples;

  ui.Gradient shader(List<ui.Color> colors) => switch (this) {
    linear => ui.Gradient.linear(
      const ui.Offset(8.5, 32.5),
      const ui.Offset(56.5, 32.5),
      colors,
      _stops,
    ),
    radial => ui.Gradient.radial(_center, 16, colors, _stops),
    sweep => ui.Gradient.sweep(_center, colors, _stops),
  };
}

Future<List<List<int>>> _measure(_Geometry geometry, ui.Color first) async {
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  canvas.drawColor(_black, ui.BlendMode.src);
  final shader = geometry.shader([first, _red]);
  canvas.drawRect(_surface, ui.Paint()..shader = shader);
  final picture = recorder.endRecording();
  final image = await picture.toImage(64, 64);
  final rgba = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  final result = [
    for (final (x, y) in geometry.samples)
      List.generate(4, (channel) => rgba.getUint8((y * 64 + x) * 4 + channel)),
  ];
  image.dispose();
  picture.dispose();
  shader.dispose();
  return result;
}

void main() {
  const opaque = [
    [64, 0, 191, 255],
    [128, 0, 128, 255],
    [191, 0, 64, 255],
  ];
  const straight = [
    [16, 0, 48, 255],
    [64, 0, 64, 255],
    [143, 0, 48, 255],
  ];
  for (final geometry in _Geometry.values) {
    test('${geometry.name} stops interpolate straight color before compositing', () async {
      for (final first in [_blue, _transparentBlue]) {
        final actual = await _measure(geometry, first);
        final expected = first == _blue ? opaque : straight;
        for (var i = 0; i < actual.length; i++) {
          for (var c = 0; c < 4; c++) {
            expect(
              actual[i][c],
              closeTo(expected[i][c], 1),
              reason:
                  '${geometry.name} first=$first quarter=${i + 1}, RGBA=$actual',
            );
          }
        }
      }
    });
  }
}
