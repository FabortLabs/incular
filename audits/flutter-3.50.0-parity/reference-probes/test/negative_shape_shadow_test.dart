import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

Future<List<int>> paintedPixels(Decoration decoration) async {
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  final painter = decoration.createBoxPainter(() {});
  painter.paint(
    canvas,
    const Offset(8, 8),
    const ImageConfiguration(
      size: Size(20, 16),
      textDirection: ui.TextDirection.ltr,
    ),
  );
  final picture = recorder.endRecording();
  final image = await picture.toImage(36, 32);
  final bytes = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  var count = 0;
  for (var y = 0; y < 32; y++) {
    for (var x = 0; x < 36; x++) {
      if (bytes.getUint8(4 * (y * 36 + x) + 3) != 0) {
        count++;
      }
    }
  }
  final result = <int>[count, bytes.getUint8(4 * (16 * 36 + 18) + 3)];
  image.dispose();
  picture.dispose();
  painter.dispose();
  return result;
}

void main() {
  test('shadow geometry at and beyond zero extent', () async {
    final cases = <String, Decoration Function(BoxShadow)>{
      'shape rectangle': (shadow) =>
          ShapeDecoration(shape: const Border(), shadows: [shadow]),
      'shape circle': (shadow) =>
          ShapeDecoration(shape: const CircleBorder(), shadows: [shadow]),
      'shape rounded rectangle': (shadow) => ShapeDecoration(
        shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.all(Radius.circular(2)),
        ),
        shadows: [shadow],
      ),
      'box rectangle': (shadow) => BoxDecoration(boxShadow: [shadow]),
      'box circle': (shadow) =>
          BoxDecoration(shape: BoxShape.circle, boxShadow: [shadow]),
      'box rounded rectangle': (shadow) => BoxDecoration(
        borderRadius: const BorderRadius.all(Radius.circular(2)),
        boxShadow: [shadow],
      ),
    };
    for (final entry in cases.entries) {
      final outcomes = <List<int>>[];
      for (final spread in <double>[-7, -8, -9, -15]) {
        outcomes.add(
          await paintedPixels(
            entry.value(
              BoxShadow(color: const Color(0xffff0000), spreadRadius: spread),
            ),
          ),
        );
      }
      final expected = switch (entry.key.split(' ').skip(1).join(' ')) {
        'rectangle' => const [
          [12, 255],
          [0, 0],
          [4, 255],
          [140, 255],
        ],
        'circle' => const [
          [4, 191],
          [0, 0],
          [4, 191],
          [90, 255],
        ],
        'rounded rectangle' => const [
          [12, 255],
          [0, 0],
          [4, 191],
          [140, 255],
        ],
        _ => throw StateError(entry.key),
      };
      expect(outcomes, expected, reason: entry.key);
    }
  });
}
