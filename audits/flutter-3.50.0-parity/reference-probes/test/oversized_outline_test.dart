import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final String name in <String>['circle', 'oval', 'stadium']) {
    test('$name oversized stroke geometry', () async {
      const area = ui.Rect.fromLTWH(4, 4, 4, 4);
      for (final double width in <double>[2, 3, 4, 8]) {
        final side = BorderSide(width: width);
        final ShapeBorder shape = switch (name) {
          'circle' => CircleBorder(side: side),
          'oval' => OvalBorder(side: side),
          _ => StadiumBorder(side: side),
        };
        final inner = shape.getInnerPath(area);
        final recorder = ui.PictureRecorder();
        final canvas = ui.Canvas(recorder);
        canvas.drawColor(const ui.Color(0xffffffff), ui.BlendMode.src);
        shape.paint(canvas, area);
        final picture = recorder.endRecording();
        final image = await picture.toImage(12, 12);
        final bytes = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        final pixels = <int>[
          for (final (x, y) in [(5, 5), (6, 6), (4, 6), (3, 6), (8, 6)])
            bytes.getUint8((y * 12 + x) * 4),
        ];
        final radius = (2 - width).abs();
        expect(
          inner.getBounds(),
          ui.Rect.fromCircle(center: area.center, radius: radius),
        );
        expect(inner.contains(area.center), radius > 0);
        if (width == 2) {
          expect(pixels[1], 0); // A thick but non-flat centerline paints.
        } else if (width == 3) {
          expect(pixels[1], inExclusiveRange(0, 255));
        } else if (width == 4 || name == 'circle') {
          expect(
            pixels,
            everyElement(255),
          ); // No zero or negative-radius circle.
        } else if (name == 'oval') {
          expect(pixels[1], 255);
          expect(pixels[3], 0); // Ordered inverted oval still paints outside.
        } else {
          expect(pixels[1], 0);
          expect(
            pixels[3],
            0,
          ); // Inverted stadium comes back with square corners.
        }
        image.dispose();
        picture.dispose();
      }
    });
  }
}
