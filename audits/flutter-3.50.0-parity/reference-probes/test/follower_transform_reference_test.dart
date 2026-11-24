import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final scale in [1.0, 2.0]) {
    testWidgets(
      'unequal linked sizes, distinct anchors, offset, scale=$scale',
      (tester) async {
        final link = LayerLink();
        const marker = ValueKey('marker');
        const boundary = ValueKey('boundary');
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: RepaintBoundary(
              key: boundary,
              child: Stack(
                children: [
                  Positioned(
                    left: 100,
                    top: 90,
                    child: Transform.scale(
                      scale: scale,
                      alignment: Alignment.topLeft,
                      child: CompositedTransformTarget(
                        link: link,
                        child: const SizedBox(width: 100, height: 60),
                      ),
                    ),
                  ),
                  Positioned(
                    left: 300,
                    top: 200,
                    child: CompositedTransformFollower(
                      link: link,
                      targetAnchor: Alignment.bottomRight,
                      followerAnchor: Alignment.topCenter,
                      offset: const Offset(7, 11),
                      child: const SizedBox(
                        key: marker,
                        width: 30,
                        height: 20,
                        child: ColoredBox(color: Color(0xFFFF5722)),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
        final expected = scale == 1
            ? const Rect.fromLTWH(192, 161, 30, 20)
            : const Rect.fromLTWH(284, 232, 60, 40);
        expect(tester.getRect(find.byKey(marker)), expected);
        final render = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(boundary),
        );
        await tester.runAsync(() async {
          final image = await render.toImage();
          try {
            final data = (await image.toByteData(
              format: ui.ImageByteFormat.rawRgba,
            ))!;
            List<int> rgba(int x, int y) {
              final start = 4 * (y * image.width + x);
              return List.generate(4, (i) => data.getUint8(start + i));
            }

            expect(
              rgba(expected.right.toInt() - 2, expected.bottom.toInt() - 2),
              [255, 87, 34, 255],
            );
            expect(
              rgba(expected.left.toInt() - 1, expected.top.toInt() + 2)[3],
              0,
            );
          } finally {
            image.dispose();
          }
        });
      },
    );
  }
}
