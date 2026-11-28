import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('Matrix minification selects a different effective filter at medium quality', () async {
    Future<List<int>> pixel(double scale, ui.FilterQuality? quality) async {
      final matrix = Float64List.fromList([
        scale,
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
      final recorder = ui.PictureRecorder();
      final canvas = ui.Canvas(recorder);
      final filter = quality == null
          ? ui.ImageFilter.matrix(matrix)
          : ui.ImageFilter.matrix(matrix, filterQuality: quality);
      canvas.saveLayer(
        const ui.Rect.fromLTWH(0, 0, 32, 16),
        ui.Paint()..imageFilter = filter,
      );
      for (var x = 0; x < 32; x++) {
        canvas.drawRect(
          ui.Rect.fromLTWH(x.toDouble(), 0, 1, 16),
          ui.Paint()
            ..isAntiAlias = false
            ..color = x % 4 == 3
                ? const ui.Color(0xff0000ff)
                : const ui.Color(0xffff0000),
        );
      }
      canvas.restore();
      final picture = recorder.endRecording();
      try {
        final image = await picture.toImage(32, 16);
        try {
          final bytes = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          final start = (8 * 32 + 3) * 4;
          return List.generate(4, (i) => bytes.getUint8(start + i));
        } finally {
          image.dispose();
        }
      } finally {
        picture.dispose();
      }
    }

    final nearest = await pixel(0.25, ui.FilterQuality.none);
    final low = await pixel(0.25, ui.FilterQuality.low);
    final medium = await pixel(0.25, ui.FilterQuality.medium);
    final defaultQuality = await pixel(0.25, null);
    expect(nearest, [255, 0, 0, 255]);
    expect(low, [255, 0, 0, 255]);
    expect(defaultQuality, medium);
    expect(medium, [191, 0, 63, 255]);
  });
}
