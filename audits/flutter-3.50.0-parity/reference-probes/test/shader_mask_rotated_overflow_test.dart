import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'rotated mask keeps overflow outside the quad, independent of alpha',
    (WidgetTester tester) async {
      const key = ValueKey<String>('capture');
      const black = <int>[0, 0, 0, 255];
      const white = <int>[255, 255, 255, 255];
      const red = <int>[255, 0, 0, 255];

      for (final (angle, alpha) in [
        (0.0, 255),
        (math.pi / 4, 255),
        (math.pi / 4, 0),
      ]) {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Align(
              alignment: Alignment.topLeft,
              child: RepaintBoundary(
                key: key,
                child: SizedBox.square(
                  dimension: 200,
                  child: ColoredBox(
                    color: const ui.Color(0xff000000),
                    child: Center(
                      child: Transform.rotate(
                        angle: angle,
                        child: ShaderMask(
                          shaderCallback: (ui.Rect bounds) {
                            expect(
                              bounds,
                              const ui.Rect.fromLTWH(0, 0, 100, 100),
                            );
                            final color = ui.Color.fromARGB(alpha, 255, 0, 0);
                            return ui.Gradient.linear(
                              bounds.topLeft,
                              bounds.bottomRight,
                              [color, color],
                            );
                          },
                          child: const SizedBox.square(
                            dimension: 100,
                            child: Stack(
                              clipBehavior: Clip.none,
                              children: [
                                Positioned(
                                  left: -20,
                                  top: 0,
                                  width: 120,
                                  height: 100,
                                  child: RepaintBoundary(
                                    child: ColoredBox(
                                      color: ui.Color(0xffffffff),
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),
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
          find.byKey(key),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          try {
            final bytes = (await image.toByteData(
              format: ui.ImageByteFormat.rawRgba,
            ))!;
            List<int> pixel(int x, int y) => List.generate(
              4,
              (c) => bytes.getUint8((y * image.width + x) * 4 + c),
            );

            List<int> localPixel(double x) {
              // The mask center is at (100,100). Sample the transformed local y=50.
              final worldX = (100 + (x - 50) * math.cos(angle)).floor();
              final worldY = (100 + (x - 50) * math.sin(angle)).floor();
              return pixel(worldX, worldY);
            }

            expect(localPixel(-30), black, reason: 'outside the child');
            expect(
              localPixel(-15),
              white,
              reason: 'painted overflow outside the quad',
            );
            expect(
              localPixel(50),
              alpha == 0 ? black : red,
              reason: 'covered child',
            );
            expect(angle == 0 ? pixel(49, 100) : pixel(64, 64), white);
            expect(
              angle == 0 ? pixel(50, 100) : pixel(64, 65),
              alpha == 0 ? black : red,
              reason: 'hard edge on the first covered pixel',
            );
          } finally {
            image.dispose();
          }
        });
      }
    },
  );
}
