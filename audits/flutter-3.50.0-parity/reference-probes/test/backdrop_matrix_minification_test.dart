import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Clipped backdrop matrix quality distinguishes Low and Medium', (
    tester,
  ) async {
    final boundary = GlobalKey();
    Future<List<int>> capture(ui.FilterQuality? quality) async {
      final matrix = Float64List.fromList([
        0.25,
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
      final filter = quality == null
          ? ui.ImageFilter.matrix(matrix)
          : ui.ImageFilter.matrix(matrix, filterQuality: quality);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: RepaintBoundary(
              key: boundary,
              child: SizedBox(
                width: 32,
                height: 16,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    Row(
                      children: List.generate(
                        32,
                        (x) => SizedBox(
                          width: 1,
                          height: 16,
                          child: ColoredBox(
                            color: x % 4 == 3
                                ? const ui.Color(0xff0000ff)
                                : const ui.Color(0xffff0000),
                          ),
                        ),
                      ),
                    ),
                    ClipRect(
                      child: BackdropFilter(
                        filter: filter,
                        child: const SizedBox.expand(),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      final captured = await tester.runAsync(() async {
        final render =
            boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary;
        final image = await render.toImage(pixelRatio: 1);
        try {
          final bytes = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          final start = (8 * 32 + 3) * 4;
          return List.generate(4, (c) => bytes.getUint8(start + c));
        } finally {
          image.dispose();
        }
      });
      return captured!;
    }

    final low = await capture(ui.FilterQuality.low);
    final medium = await capture(ui.FilterQuality.medium);
    final implicit = await capture(null);
    expect(low, [255, 0, 0, 255]);
    expect(medium, [191, 0, 63, 255]);
    expect(implicit, medium);
  });
}
