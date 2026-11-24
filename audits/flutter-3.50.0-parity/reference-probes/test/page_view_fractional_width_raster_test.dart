import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('fractional width does not stretch the last page marker', (
    tester,
  ) async {
    final boundary = GlobalKey();
    final controller = PageController();
    addTearDown(controller.dispose);
    const colors = [Color(0xff0000ff), Color(0xff00ff00), Color(0xffff0000)];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: RepaintBoundary(
            key: boundary,
            child: SizedBox(
              width: 512,
              height: 192,
              child: ColoredBox(
                color: const Color(0xff000000),
                child: Center(
                  child: SizedBox(
                    width: 411.42857142857144,
                    height: 192,
                    child: PageView(
                      controller: controller,
                      physics: const ClampingScrollPhysics(),
                      children: colors
                          .map(
                            (color) => Center(
                              child: SizedBox(
                                width: 60,
                                height: 22,
                                child: ColoredBox(color: color),
                              ),
                            ),
                          )
                          .toList(),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    controller.animateToPage(
      2,
      duration: const Duration(milliseconds: 300),
      curve: Curves.ease,
    );
    await tester.pumpAndSettle();
    expect(controller.page, 2);
    final render = tester.renderObject<RenderRepaintBoundary>(
      find.byKey(boundary),
    );
    final data = (await tester.runAsync(() async {
      final image = await render.toImage(pixelRatio: 1);
      try {
        expect((image.width, image.height), (512, 192));
        return (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
      } finally {
        image.dispose();
      }
    }))!;
    for (final (x, y, red) in [
      (256, 96, true),
      (226, 85, true),
      (285, 106, true),
      (225, 85, false),
      (286, 106, false),
      (226, 84, false),
      (285, 107, false),
    ]) {
      final index = (y * 512 + x) * 4;
      expect(List.generate(4, (i) => data.getUint8(index + i)), [
        red ? 255 : 0,
        0,
        0,
        255,
      ], reason: 'marker pixel ($x,$y)');
    }
  });
}
