import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a retained mask follows translation and reflection', (
    WidgetTester tester,
  ) async {
    const captureKey = ValueKey<String>('capture');
    final masked = RepaintBoundary(
      child: ShaderMask(
        shaderCallback: (ui.Rect bounds) {
          expect(bounds, const ui.Rect.fromLTWH(0, 0, 16, 16));
          return ui.Gradient.linear(
            bounds.topLeft,
            bounds.topRight,
            const <ui.Color>[
              ui.Color(0xffff0000),
              ui.Color(0xffff0000),
              ui.Color(0xff0000ff),
              ui.Color(0xff0000ff),
            ],
            const <double>[0, 0.5, 0.5, 1],
          );
        },
        child: const SizedBox.square(
          dimension: 16,
          child: ColoredBox(color: ui.Color(0xffffffff)),
        ),
      ),
    );

    const black = <int>[0, 0, 0, 255];
    const red = <int>[255, 0, 0, 255];
    const blue = <int>[0, 0, 255, 255];
    for (final (x, flip) in [(8, false), (32, false), (32, true), (8, true)]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: RepaintBoundary(
              key: captureKey,
              child: SizedBox(
                width: 64,
                height: 48,
                child: ColoredBox(
                  color: const ui.Color(0xff000000),
                  child: Stack(
                    children: [
                      Positioned(
                        left: 0,
                        top: 8,
                        child: Transform.translate(
                          offset: ui.Offset(x.toDouble(), 0),
                          child: Transform.flip(flipX: flip, child: masked),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      final boundary = tester.renderObject<RenderRepaintBoundary>(
        find.byKey(captureKey),
      );
      await tester.runAsync(() async {
        final image = await boundary.toImage();
        try {
          final bytes = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          List<int> pixel(int x, int y) => List.generate(
            4,
            (i) => bytes.getUint8((y * image.width + x) * 4 + i),
          );
          expect(pixel(x + 3, 15), flip ? blue : red);
          expect(pixel(x + 12, 15), flip ? red : blue);
          expect(pixel((x == 8 ? 32 : 8) + 3, 15), black);
          expect(pixel(x - 1, 15), black);
          expect(pixel(x + 16, 15), black);
        } finally {
          image.dispose();
        }
      });
    }
  });
}
