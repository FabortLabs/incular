import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

const _side = 64;
const _samples = [(0, 0), (1, 10), (62, 10), (1, 53), (62, 53), (32, 32)];
const _black = [0, 0, 0, 255];
const _white = [255, 255, 255, 255];

Future<List<List<int>>> _paint(BorderRadius? radius) async {
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  canvas.drawColor(const ui.Color(0xff000000), ui.BlendMode.src);
  final painter = BoxDecoration(
    color: const ui.Color(0xffffffff),
    borderRadius: radius,
  ).createBoxPainter();
  painter.paint(
    canvas,
    ui.Offset.zero,
    const ImageConfiguration(size: ui.Size(64, 64)),
  );
  final picture = recorder.endRecording();
  final image = await picture.toImage(_side, _side);
  final rgba = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  final result = [
    for (final (x, y) in _samples)
      List.generate(
        4,
        (channel) => rgba.getUint8((y * _side + x) * 4 + channel),
      ),
  ];
  painter.dispose();
  image.dispose();
  picture.dispose();
  return result;
}

void main() {
  test(
    'BoxDecoration solid fill preserves both elliptical corner radii',
    () async {
      expect(await _paint(null), [
        _white,
        _white,
        _white,
        _white,
        _white,
        _white,
      ]);
      expect(await _paint(const BorderRadius.all(Radius.circular(24))), [
        _black,
        _black,
        _black,
        _black,
        _black,
        _white,
      ]);
      expect(await _paint(const BorderRadius.all(Radius.elliptical(24, 8))), [
        _black,
        _white,
        _white,
        _white,
        _white,
        _white,
      ]);
    },
  );
}
