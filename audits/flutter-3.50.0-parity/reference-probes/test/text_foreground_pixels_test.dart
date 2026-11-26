import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'Foreground paint wins merge and switches cleanly during interpolation',
    () async {
      const red = ui.Color(0xffff0000);
      const blue = ui.Color(0xff0000ff);
      const redColor = TextStyle(color: red, fontSize: 24);
      const blueColor = TextStyle(color: blue, fontSize: 24);
      final redPaint = TextStyle(
        foreground: ui.Paint()..color = red,
        fontSize: 24,
      );
      final bluePaint = TextStyle(
        foreground: ui.Paint()..color = blue,
        fontSize: 24,
      );

      Future<void> check(TextStyle style, ui.Color expected) async {
        final painter = TextPainter(
          text: TextSpan(text: 'M', style: style),
          textDirection: ui.TextDirection.ltr,
        )..layout();
        final recorder = ui.PictureRecorder();
        painter.paint(ui.Canvas(recorder), const ui.Offset(5, 5));
        final picture = recorder.endRecording();
        final image = await picture.toImage(48, 48);
        final pixels = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        var opaquePixels = 0;
        for (var i = 0; i < pixels.lengthInBytes; i += 4) {
          if (pixels.getUint8(i + 3) != 255) continue;
          opaquePixels++;
          expect(
            pixels.getUint32(i),
            (expected.toARGB32() << 8 | 255) & 0xffffffff,
          );
        }
        expect(opaquePixels, greaterThan(0));
        image.dispose();
        picture.dispose();
        painter.dispose();
      }

      await check(redPaint.merge(blueColor), red);
      await check(redPaint.apply(color: blue), red);
      await check(TextStyle.lerp(redColor, bluePaint, .25)!, red);
      await check(TextStyle.lerp(redPaint, bluePaint, .25)!, red);
      await check(TextStyle.lerp(redPaint, bluePaint, .5)!, blue);
      await check(TextStyle.lerp(redColor, bluePaint, .75)!, blue);
    },
  );
}
