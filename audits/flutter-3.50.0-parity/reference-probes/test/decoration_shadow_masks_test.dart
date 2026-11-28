import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

Future<List<List<int>>> samples(
  Decoration decoration, {
  bool backdrop = false,
}) async {
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  if (backdrop) {
    canvas.drawColor(const Color(0xff0000ff), ui.BlendMode.src);
  }
  final painter = decoration.createBoxPainter(() {});
  painter.paint(
    canvas,
    const Offset(15, 15),
    const ImageConfiguration(
      size: Size(20, 20),
      textDirection: ui.TextDirection.ltr,
    ),
  );
  final picture = recorder.endRecording();
  final image = await picture.toImage(50, 50);
  final data = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  final samples = <List<int>>[
    for (final x in [14, 15, 25])
      [for (var i = 0; i < 4; i++) data.getUint8(4 * (25 * 50 + x) + i)],
  ];
  painter.dispose();
  image.dispose();
  picture.dispose();
  return samples;
}

void main() {
  test('box and shape blur masks share edge and alpha semantics', () async {
    final previous = debugDisableShadows;
    debugDisableShadows = false;
    try {
      for (final shape in [false, true]) {
        Decoration decoration(
          ui.BlurStyle style, {
          double radius = 4,
          int alpha = 255,
        }) {
          final shadow = BoxShadow(
            color: Color.fromARGB(alpha, 255, 0, 0),
            blurRadius: radius,
            blurStyle: style,
          );
          return shape
              ? ShapeDecoration(shape: const Border(), shadows: [shadow])
              : BoxDecoration(boxShadow: [shadow]);
        }

        final normal = await samples(decoration(ui.BlurStyle.normal));
        expect(normal[0][3], inInclusiveRange(1, 254));
        expect(normal[1][3], inInclusiveRange(1, 254));
        for (final style in ui.BlurStyle.values) {
          final opaque = await samples(decoration(style));
          final translucent = await samples(decoration(style, alpha: 128));
          final expected = switch (style) {
            ui.BlurStyle.normal => [normal[0][3], normal[1][3], normal[2][3]],
            ui.BlurStyle.solid => [normal[0][3], 255, 255],
            ui.BlurStyle.outer => [normal[0][3], 0, 0],
            ui.BlurStyle.inner => [0, normal[1][3], normal[2][3]],
          };
          for (var i = 0; i < 3; i++) {
            expect(
              opaque[i][3],
              closeTo(
                expected[i],
                expected[i] == 0 || expected[i] == 255 ? 0 : 8,
              ),
              reason: '$shape $style $i',
            );
            expect(
              translucent[i][3],
              closeTo(opaque[i][3] * 128 / 255, 1),
              reason: '$shape $style $i',
            );
          }
          final zero = await samples(decoration(style, radius: 0));
          expect(
            zero.map((sample) => sample[3]).toList(),
            [0, 255, 255],
            reason:
                'zero radius must draw the unfiltered shadow for $shape $style',
          );
        }
        final outerOverBlue = await samples(
          decoration(ui.BlurStyle.outer),
          backdrop: true,
        );
        expect(outerOverBlue[2], [0, 0, 255, 255]);
        expect(outerOverBlue[0][0], greaterThan(0));
        expect(outerOverBlue[0][3], 255);
      }
    } finally {
      debugDisableShadows = previous;
    }
  });
}
