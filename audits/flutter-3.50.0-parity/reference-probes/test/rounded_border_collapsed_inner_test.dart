import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('rounded inside border handles a shrinking interior', (
    tester,
  ) async {
    final capture = GlobalKey();
    for (final (size, center) in [
      (const Size(5, 4), 255),
      (const Size(6, 6), 0),
      (const Size(6, 7), 0),
      (const Size(7, 7), 255),
    ]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: RepaintBoundary(
              key: capture,
              child: SizedBox.fromSize(
                size: size,
                child: ColoredBox(
                  color: const Color(0xffffffff),
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      borderRadius: const BorderRadius.all(
                        Radius.elliptical(2, 1),
                      ),
                      border: Border.all(
                        color: const Color(0xff000000),
                        width: 3,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      final boundary = tester.renderObject<RenderRepaintBoundary>(
        find.byKey(capture),
      );
      await tester.runAsync(() async {
        final image = await boundary.toImage();
        try {
          final pixels = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          List<int> pixel(int x, int y) {
            final offset = (image.width * y + x) * 4;
            return List.generate(4, (index) => pixels.getUint8(offset + index));
          }

          expect(pixel(0, 2), [0, 0, 0, 255], reason: '$size border');
          expect(pixel(image.width ~/ 2, image.height ~/ 2), [
            center,
            center,
            center,
            255,
          ], reason: '$size center');
        } finally {
          image.dispose();
        }
      });
    }
  });
}
