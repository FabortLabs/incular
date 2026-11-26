import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'rounded outline distinguishes inverted from flat inner bounds',
    () async {
      const border = RoundedRectangleBorder(side: BorderSide(width: 3));
      for (final (int width, int height, bool hole) in <(int, int, bool)>[
        (5, 4, true),
        (6, 6, false),
        (6, 7, false),
        (7, 7, true),
      ]) {
        final recorder = ui.PictureRecorder();
        final canvas = ui.Canvas(recorder);
        canvas.drawColor(const ui.Color(0xffffffff), ui.BlendMode.src);
        border.paint(
          canvas,
          ui.Rect.fromLTWH(0, 0, width.toDouble(), height.toDouble()),
        );
        final picture = recorder.endRecording();
        final image = await picture.toImage(width, height);
        final bytes = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        final index = ((height ~/ 2) * width + width ~/ 2) * 4;
        expect(
          bytes.getUint8(index),
          hole ? 255 : 0,
          reason: '${width}x$height',
        );
        expect(bytes.getUint8(0), 0);
        image.dispose();
        picture.dispose();
      }
    },
  );
}
