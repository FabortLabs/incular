import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'a small destination blend does not crop distant transformed paint',
    () async {
      Future<List<int>> capture({required bool blend}) async {
        final recorder = ui.PictureRecorder();
        final canvas = ui.Canvas(recorder);
        const blendBounds = ui.Rect.fromLTRB(4, 4, 8, 8);
        canvas.drawRect(
          blendBounds,
          ui.Paint()..color = const ui.Color(0xffff0000),
        );
        canvas.save();
        canvas.translate(0, 0.25);
        canvas.scale(2);
        canvas.drawLine(
          const ui.Offset(8, 8),
          const ui.Offset(24, 8),
          ui.Paint()
            ..color = const ui.Color(0xffffffff)
            ..strokeWidth = 0,
        );
        canvas.restore();
        if (blend) {
          canvas.saveLayer(
            blendBounds,
            ui.Paint()..blendMode = ui.BlendMode.multiply,
          );
          canvas.drawRect(
            blendBounds,
            ui.Paint()..color = const ui.Color(0xff0000ff),
          );
          canvas.restore();
        }
        final picture = recorder.endRecording();
        final image = await picture.toImage(64, 64);
        try {
          final pixels = (await image.toByteData())!;
          List<int> rgba(int x, int y) => List<int>.generate(
            4,
            (channel) => pixels.getUint8((y * 64 + x) * 4 + channel),
          );
          expect(rgba(0, 0), [0, 0, 0, 0]);
          expect(rgba(6, 6), blend ? [0, 0, 0, 255] : [255, 0, 0, 255]);
          return [14, 15, 16, 17, 18].map((y) => rgba(32, y)[3]).toList();
        } finally {
          image.dispose();
          picture.dispose();
        }
      }

      final plain = await capture(blend: false);
      expect(plain.any((alpha) => alpha > 0), isTrue);
      expect(await capture(blend: true), plain);
    },
  );
}
