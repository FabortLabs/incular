import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'offset and spread move the blur masks, not the painted box edges',
    () async {
      const backdrop = Color(0xffffc107);
      const color = Color(0xfff44336);
      const black = Color(0xff000000);
      final previous = debugDisableShadows;
      debugDisableShadows = false;
      try {
        for (final style in ui.BlurStyle.values) {
          final decoration = BoxDecoration(
            color: black,
            boxShadow: [
              BoxShadow(
                color: color,
                offset: const Offset(4, 4),
                blurRadius: 16,
                spreadRadius: 2,
                blurStyle: style,
              ),
            ],
          );
          final recorder = ui.PictureRecorder();
          final canvas = ui.Canvas(recorder)
            ..drawColor(backdrop, ui.BlendMode.src);
          final painter = decoration.createBoxPainter(() {});
          painter.paint(
            canvas,
            const Offset(32, 32),
            const ImageConfiguration(size: Size.square(64)),
          );
          final picture = recorder.endRecording();
          final image = await picture.toImage(128, 128);
          final data = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          int pixel(int x, int y) => data.getUint32((y * 128 + x) * 4);
          const amber = 0xffc107ff;
          const red = 0xf44336ff;
          expect(pixel(32, 64), 0x000000ff, reason: '$style original box edge');
          expect(
            pixel(96, 64),
            isNot(0x000000ff),
            reason: '$style never paints a black border',
          );
          expect(pixel(0, 0), amber);
          final insideShadow = pixel(98, 64);
          final outsideShadow = pixel(104, 64);
          switch (style) {
            case ui.BlurStyle.normal:
              expect(insideShadow, isNot(amber));
              expect(outsideShadow, isNot(amber));
            case ui.BlurStyle.solid:
              expect(insideShadow, red);
              expect(outsideShadow, isNot(amber));
            case ui.BlurStyle.outer:
              expect(insideShadow, amber);
              expect(outsideShadow, isNot(amber));
            case ui.BlurStyle.inner:
              expect(insideShadow, isNot(amber));
              expect(outsideShadow, amber);
          }
          painter.dispose();
          image.dispose();
          picture.dispose();
        }
      } finally {
        debugDisableShadows = previous;
      }
    },
  );
}
