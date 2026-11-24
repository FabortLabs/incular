import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _colors = [Color(0xffff0000), Color(0xff00ff00), Color(0xff0000ff)];
const _rgba = [
  [255, 0, 0, 255],
  [0, 255, 0, 255],
  [0, 0, 255, 255],
];

void main() {
  testWidgets('retained index changes replace only the selected pixel region', (
    tester,
  ) async {
    final boundary = GlobalKey();
    final children = _colors
        .map(
          (color) =>
              SizedBox(width: 28, height: 20, child: ColoredBox(color: color)),
        )
        .toList();
    for (final i in <int?>[0, null, 1, 2, null, 0]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: RepaintBoundary(
              key: boundary,
              child: SizedBox(
                width: 64,
                height: 48,
                child: ColoredBox(
                  color: const Color(0xff000000),
                  child: Center(
                    child: IndexedStack(index: i, children: children),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      final render = tester.renderObject<RenderRepaintBoundary>(
        find.byKey(boundary),
      );
      final bytes = (await tester.runAsync(() async {
        final image = await render.toImage(pixelRatio: 1);
        try {
          expect((image.width, image.height), (64, 48));
          return (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
        } finally {
          image.dispose();
        }
      }))!;
      for (final (x, y, inside) in [
        (32, 24, true),
        (18, 14, true),
        (45, 33, true),
        (17, 14, false),
        (46, 33, false),
        (18, 13, false),
        (45, 34, false),
      ]) {
        final offset = (y * 64 + x) * 4;
        expect(
          List.generate(4, (k) => bytes.getUint8(offset + k)),
          inside && i != null ? _rgba[i] : const [0, 0, 0, 255],
          reason: 'selected index $i, pixel ($x,$y)',
        );
      }
    }
  });
}
