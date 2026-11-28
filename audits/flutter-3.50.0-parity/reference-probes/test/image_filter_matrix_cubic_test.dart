import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'High matrix sampling uses cubic edge weights rather than Low',
    () async {
      Future<List<List<int>>> capture(ui.FilterQuality quality) async {
        final recorder = ui.PictureRecorder();
        final canvas = ui.Canvas(recorder);
        final matrix = Float64List.fromList([
          2,
          0,
          0,
          0,
          0,
          1,
          0,
          0,
          0,
          0,
          1,
          0,
          0,
          0,
          0,
          1,
        ]);
        canvas.saveLayer(
          const ui.Rect.fromLTWH(0, 0, 4, 4),
          ui.Paint()
            ..imageFilter = ui.ImageFilter.matrix(
              matrix,
              filterQuality: quality,
            ),
        );
        canvas.drawRect(
          const ui.Rect.fromLTWH(0, 0, 2, 4),
          ui.Paint()..color = const ui.Color(0xffff0000),
        );
        canvas.drawRect(
          const ui.Rect.fromLTWH(2, 0, 2, 4),
          ui.Paint()..color = const ui.Color(0xff0000ff),
        );
        canvas.restore();
        final picture = recorder.endRecording();
        try {
          final image = await picture.toImage(10, 4);
          try {
            final bytes = (await image.toByteData(
              format: ui.ImageByteFormat.rawRgba,
            ))!;
            return [2, 3, 4, 5, 8].map((x) {
              final start = (2 * 10 + x) * 4;
              return List.generate(
                4,
                (channel) => bytes.getUint8(start + channel),
              );
            }).toList();
          } finally {
            image.dispose();
          }
        } finally {
          picture.dispose();
        }
      }

      final low = await capture(ui.FilterQuality.low);
      final high = await capture(ui.FilterQuality.high);
      expect(low, [
        [255, 0, 0, 255],
        [191, 0, 64, 255],
        [64, 0, 191, 255],
        [0, 0, 255, 255],
        [0, 0, 0, 0],
      ]);
      expect(high, [
        [255, 0, 0, 255],
        [193, 0, 62, 255],
        [62, 0, 193, 255],
        [0, 0, 255, 255],
        [0, 0, 0, 0],
      ]);
    },
  );
}
