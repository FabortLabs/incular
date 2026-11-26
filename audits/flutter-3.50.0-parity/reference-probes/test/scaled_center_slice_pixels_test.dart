import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('paintImage with a scaled center exceeding the image', () async {
    const red = ui.Color(0xffff0000);
    const blue = ui.Color(0xff0000ff);
    const green = ui.Color(0xff00ff00);
    final source = ui.PictureRecorder();
    final sourceCanvas = ui.Canvas(source);
    sourceCanvas.drawRect(
      const ui.Rect.fromLTWH(0, 0, 150, 300),
      ui.Paint()..color = red,
    );
    sourceCanvas.drawRect(
      const ui.Rect.fromLTWH(150, 0, 150, 300),
      ui.Paint()..color = blue,
    );
    final sourcePicture = source.endRecording();
    final image = sourcePicture.toImageSync(300, 300);
    addTearDown(sourcePicture.dispose);
    addTearDown(image.dispose);

    for (final slice in <ui.Rect>[
      const ui.Rect.fromLTRB(50, 40, 250, 260),
      const ui.Rect.fromLTRB(50, 40, 250, 80),
      const ui.Rect.fromLTRB(50, 40, 90, 260),
      const ui.Rect.fromLTRB(-1, 40, 39, 80),
    ]) {
      final target = ui.PictureRecorder();
      final canvas = ui.Canvas(target);
      canvas.drawColor(green, ui.BlendMode.src);
      paintImage(
        canvas: canvas,
        rect: const ui.Rect.fromLTRB(10, 20, 430, 420),
        image: image,
        scale: 2,
        centerSlice: slice,
        filterQuality: ui.FilterQuality.none,
      );
      final picture = target.endRecording();
      final targetImage = picture.toImageSync(450, 440);
      addTearDown(picture.dispose);
      addTearDown(targetImage.dispose);
      final bytes = (await targetImage.toByteData())!;
      int pixel(int x, int y) => bytes.getUint32((y * 450 + x) * 4);

      // First/last painted rows, their neighbors, and the red/blue seam.
      for (final y in <int>[19, 20, 200, 419, 420]) {
        for (final x in <int>[9, 10, 50, 219, 220, 300, 429, 430]) {
          final inside = x >= 10 && x < 430 && y >= 20 && y < 420;
          expect(
            pixel(x, y),
            !inside ? 0x00ff00ff : (x < 220 ? 0xff0000ff : 0x0000ffff),
            reason: 'slice $slice at ($x, $y)',
          );
        }
      }
    }
  });
}
