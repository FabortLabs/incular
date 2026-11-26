import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

Future<ByteData> raster(TextStyle style, String value) async {
  final paragraph = TextPainter(
    text: TextSpan(text: value, style: style),
    textDirection: ui.TextDirection.ltr,
    textAlign: ui.TextAlign.right,
  )..layout(minWidth: 90, maxWidth: 90);
  final recorder = ui.PictureRecorder();
  paragraph.paint(ui.Canvas(recorder), const ui.Offset(7, 11));
  final picture = recorder.endRecording();
  final image = await picture.toImage(110, 80);
  final data = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  paragraph.dispose();
  image.dispose();
  picture.dispose();
  return data;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const red = ui.Color(0xffff0000);
  const blue = ui.Color(0xff0000ff);
  const transparent = ui.Color(0x00000000);
  const plain = TextStyle(color: transparent, fontFamily: 'Ahem', fontSize: 20);
  const simple = TextStyle(backgroundColor: red);

  test(
    'Background pixels stop at the line advance and track its alignment',
    () async {
      final data = await raster(plain.merge(simple), 'AA\nA');
      int pixel(int x, int y) => data.getUint32((110 * y + x) * 4);
      const redRgba = 0xff0000ff;
      for (final (row, left, right) in [(16, 57, 97), (36, 77, 97)]) {
        expect(pixel(left - 1, row), 0);
        expect(pixel(left, row), redRgba);
        expect(pixel(right - 1, row), redRgba);
        expect(pixel(right, row), 0);
      }
      expect(pixel(90, 10), 0);
      expect(pixel(90, 11), redRgba);
      expect(pixel(90, 50), redRgba);
      expect(pixel(90, 51), 0);
    },
  );

  test(
    'Background paint takes precedence and switches at the midpoint',
    () async {
      final redPaint = TextStyle(background: ui.Paint()..color = red);
      final bluePaint = TextStyle(background: ui.Paint()..color = blue);
      const blueColor = TextStyle(backgroundColor: blue);
      for (final (style, expected) in [
        (redPaint.merge(blueColor), 0xff0000ff),
        (redPaint.apply(backgroundColor: blue), 0xff0000ff),
        (TextStyle.lerp(redPaint, bluePaint, .25)!, 0xff0000ff),
        (TextStyle.lerp(simple, bluePaint, .5)!, 0x0000ffff),
      ]) {
        final data = await raster(plain.merge(style), 'A');
        expect(data.getUint32((110 * 16 + 90) * 4), expected);
      }
    },
  );
  test(
    'Explicit run height also covers the added vertical background area',
    () async {
      final style = plain.merge(simple).copyWith(height: 2);
      final data = await raster(style, 'A');
      int pixel(int y) => data.getUint32((110 * y + 90) * 4);
      expect(pixel(10), 0);
      expect(pixel(11), 0xff0000ff);
      expect(pixel(50), 0xff0000ff);
      expect(pixel(51), 0);
    },
  );
}
