import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'same glyph and ppem produce different ink at different axis positions',
    () async {
      final fixture = Platform.environment['INCULAR_FLUTTER_VARIABLE_FONT']!;
      final source = await File(fixture).readAsBytes();
      const family = 'ParityRobotoSlabMask';
      final loader = FontLoader(family)
        ..addFont(Future<ByteData>.value(ByteData.sublistView(source)));
      await loader.load();

      TextStyle style(double weight) => TextStyle(
        fontFamily: family,
        fontSize: 56,
        color: const ui.Color(0xFFFFFFFF),
        fontVariations: [ui.FontVariation.weight(weight)],
      );
      Future<Uint8List> ink(TextStyle style) async {
        final builder =
            ui.ParagraphBuilder(
                ui.ParagraphStyle(textDirection: ui.TextDirection.ltr),
              )
              ..pushStyle(style.getTextStyle())
              ..addText('H');
        final paragraph = builder.build()
          ..layout(const ui.ParagraphConstraints(width: 90));
        final recorder = ui.PictureRecorder();
        ui.Canvas(recorder)
            .drawParagraph(paragraph, const ui.Offset(10.25, 10.75));
        final picture = recorder.endRecording();
        final image = await picture.toImage(100, 100);
        try {
          final bytes = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          return Uint8List.fromList([
            for (int i = 3; i < bytes.lengthInBytes; i += 4) bytes.getUint8(i),
          ]);
        } finally {
          image.dispose();
          picture.dispose();
        }
      }

      final light = style(100);
      final heavy = style(900);
      final low = await ink(light);
      final mid = await ink(TextStyle.lerp(light, heavy, 0.5)!);
      final high = await ink(heavy);
      expect(mid, orderedEquals(await ink(style(500))));
      int area(Uint8List pixels) =>
          pixels.fold(0, (int sum, int pixel) => sum + pixel);
      expect(area(low), lessThan(area(mid)));
      expect(area(mid), lessThan(area(high)));
    },
  );
}
