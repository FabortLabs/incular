import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Stack clips layout overflow, not paint-only overflow', (
    tester,
  ) async {
    final key = GlobalKey();
    const red = SizedBox(
      width: 20,
      height: 20,
      child: ColoredBox(color: Color(0xffff0000)),
    );
    for (final (positioned, clip) in [
      (false, Clip.hardEdge),
      (true, Clip.hardEdge),
      (false, Clip.none),
      (true, Clip.none),
    ]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: RepaintBoundary(
              key: key,
              child: SizedBox(
                width: 100,
                height: 100,
                child: ColoredBox(
                  color: const Color(0xff000000),
                  child: Center(
                    child: SizedBox(
                      width: 40,
                      height: 40,
                      child: Stack(
                        clipBehavior: clip,
                        children: [
                          if (positioned)
                            const Positioned(
                              left: 30,
                              top: 0,
                              width: 20,
                              height: 20,
                              child: red,
                            )
                          else
                            Transform.translate(
                              offset: const Offset(30, 0),
                              child: red,
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
      );
      final render = tester.renderObject<RenderRepaintBoundary>(
        find.byKey(key),
      );
      final pixels = (await tester.runAsync(() async {
        final image = await render.toImage(pixelRatio: 1);
        try {
          expect((image.width, image.height), (100, 100));
          return (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
        } finally {
          image.dispose();
        }
      }))!;
      for (final (x, r) in [
        (59, 0),
        (65, 255),
        (75, positioned && clip == Clip.hardEdge ? 0 : 255),
        (80, 0),
      ]) {
        final offset = (40 * 100 + x) * 4;
        expect(List.generate(4, (i) => pixels.getUint8(offset + i)), [
          r,
          0,
          0,
          255,
        ], reason: 'positioned=$positioned, clip=$clip, x=$x');
      }
    }
  });
}
