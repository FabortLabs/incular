import 'dart:ui' as ui;

import 'package:flutter/painting.dart' show Border;
import 'package:flutter_test/flutter_test.dart';

const white = ui.Color(0xffffffff),
    red = ui.Color(0xffff0000),
    blue = ui.Color(0xff0000ff),
    green = ui.Color(0xff00ff00);
const box = ui.Rect.fromLTWH(8, 8, 48, 48);
void main() {
  test('neutral raster audit', () async {
    for (final dpr in [1, 2]) {
      for (final kind in ['border', 'stroke', 'ordinary', 'blend', 'filter']) {
        final recorder = ui.PictureRecorder();
        final c = ui.Canvas(recorder)..scale(dpr.toDouble());
        final isBorder = kind == 'border' || kind == 'stroke';
        c.drawColor(isBorder ? white : blue, ui.BlendMode.src);
        final p = ui.Path()..addRect(box);
        if (kind == 'border') {
          Border.all(color: red, width: 0).paint(c, box);
        } else {
          final paint = ui.Paint()..color = red;
          if (kind == 'stroke') {
            paint.style = ui.PaintingStyle.stroke;
            paint.strokeWidth = 0;
          }
          if (kind == 'blend') {
            paint.blendMode = ui.BlendMode.dstOver;
          }
          if (kind == 'filter') {
            paint.colorFilter = const ui.ColorFilter.mode(
              green,
              ui.BlendMode.srcIn,
            );
          }
          c.drawPath(p, paint);
        }
        final picture = recorder.endRecording();
        final image = await picture.toImage(64 * dpr, 64 * dpr);
        final bytes = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        List<int> pixel(int x, int y) => List.generate(
          4,
          (i) => bytes.getUint8((y * image.width + x) * 4 + i),
        );
        final center = pixel(32 * dpr, 32 * dpr);
        final edge = [
          for (var y = 8 * dpr - 2; y <= 8 * dpr + 2; y++)
            pixel(32 * dpr, y)[1],
        ];
        print('$kind $dpr center=$center edgeGreen=$edge');
        expect(
          center,
          isBorder
              ? [255, 255, 255, 255]
              : kind == 'blend'
              ? [0, 0, 255, 255]
              : kind == 'filter'
              ? [0, 255, 0, 255]
              : [255, 0, 0, 255],
        );
        if (isBorder) {
          expect(pixel(2 * dpr, 2 * dpr), [255, 255, 255, 255]);
          expect(
            edge.map((g) => (255 - g) / 255).reduce((a, b) => a + b),
            closeTo(1, 0.15),
          );
        }
        image.dispose();
        picture.dispose();
      }
    }
  });
}
