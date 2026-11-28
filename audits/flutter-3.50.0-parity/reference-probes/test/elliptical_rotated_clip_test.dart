import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _white = Color(0xffffffff);
const _blue = Color(0xff0000ff);
const _radii = BorderRadius.only(
  topLeft: Radius.elliptical(10, 20),
  topRight: Radius.elliptical(5, 30),
  bottomLeft: Radius.elliptical(2.5, 12),
  bottomRight: Radius.elliptical(15, 6),
);

void main() {
  testWidgets('rotated elliptical clips keep the vertical radii in raster output', (
    tester,
  ) async {
    final key = GlobalKey();
    await tester.pumpWidget(
      Center(
        child: RepaintBoundary(
          key: key,
          child: SizedBox.square(
            dimension: 160,
            child: ColoredBox(
              color: _white,
              child: Center(
                child: SizedBox.square(
                  dimension: 100,
                  child: Transform.rotate(
                    angle: 1,
                    child: const ClipRRect(
                      borderRadius: _radii,
                      child: ColoredBox(color: _blue),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    final boundary = tester.renderObject<RenderRepaintBoundary>(find.byKey(key));
    await tester.runAsync(() async {
      const scale = 4;
      final image = await boundary.toImage(pixelRatio: scale.toDouble());
      try {
        final raw = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        for (final (x, y, inside) in [
          (2.75, 4.25, false),
          (5.0, 5.0, true),
          (98.0, 2.0, false),
          (97.0, 10.0, true),
          (50.0, 0.5, true),
          (50.0, 50.0, true),
        ]) {
          final dx = x - 50;
          final dy = y - 50;
          final px = ((80 + math.cos(1) * dx - math.sin(1) * dy) * scale).floor();
          final py = ((80 + math.sin(1) * dx + math.cos(1) * dy) * scale).floor();
          final i = (py * image.width + px) * 4;
          final rgba = List.generate(4, (channel) => raw.getUint8(i + channel));
          expect(rgba, inside ? [0, 0, 255, 255] : [255, 255, 255, 255],
            reason: 'clip-local point ($x, $y)');
        }
      } finally {
        image.dispose();
      }
    });
  });
}
