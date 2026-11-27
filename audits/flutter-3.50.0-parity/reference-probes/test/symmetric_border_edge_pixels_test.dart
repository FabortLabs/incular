import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('symmetric vertical border leaves the horizontal pixels empty', (
    tester,
  ) async {
    const boundary = ValueKey('boundary');
    await tester.pumpWidget(
      const Center(
        child: RepaintBoundary(
          key: boundary,
          child: DecoratedBox(
            decoration: BoxDecoration(
              border: Border.symmetric(
                vertical: BorderSide(color: Color(0xffff0000), width: 3),
              ),
            ),
            child: SizedBox(width: 40, height: 30),
          ),
        ),
      ),
    );
    final render = tester.renderObject<RenderRepaintBoundary>(
      find.byKey(boundary),
    );
    await tester.runAsync(() async {
      final image = await render.toImage();
      try {
        final data = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        List<int> pixel(int x, int y) => List<int>.generate(
          4,
          (channel) => data.getUint8((y * image.width + x) * 4 + channel),
        );
        expect(pixel(0, 15), [255, 0, 0, 255]);
        expect(pixel(39, 15), [255, 0, 0, 255]);
        expect(pixel(20, 0), [0, 0, 0, 0]);
        expect(pixel(20, 29), [0, 0, 0, 0]);
        expect(pixel(3, 15), [0, 0, 0, 0]);
        expect(pixel(36, 15), [0, 0, 0, 0]);
      } finally {
        image.dispose();
      }
    });
  });
}
