import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('symmetric border does not paint an omitted pair of sides', (
    WidgetTester tester,
  ) async {
    const side = BorderSide(color: Color(0xFFDC143C), width: 2);
    for (final vertical in [true, false]) {
      final border = vertical
          ? const Border.symmetric(vertical: side)
          : const Border.symmetric(horizontal: side);
      final recorder = ui.PictureRecorder();
      final canvas = Canvas(recorder);
      border.paint(canvas, const Rect.fromLTWH(2, 2, 20, 16));
      final bytes = (await tester.runAsync(() async {
        final picture = recorder.endRecording();
        final image = await picture.toImage(24, 20);
        final pixels = await image.toByteData();
        image.dispose();
        picture.dispose();
        return pixels;
      }))!;
      bool painted(int x, int y) => bytes.getUint8((y * 24 + x) * 4 + 3) > 0;
      expect(painted(2, 10), vertical);
      expect(painted(21, 10), vertical);
      expect(painted(12, 2), !vertical);
      expect(painted(12, 17), !vertical);
      expect(painted(12, 10), isFalse);
      expect(painted(1, 10), isFalse);
      expect(painted(12, 1), isFalse);
    }
  });
}
