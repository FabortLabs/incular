import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _black = [0, 0, 0, 255];
const _white = [255, 255, 255, 255];

Future<({List<int> corner, List<int> center, int partial, int redFringe})>
_capture(
  WidgetTester tester,
  Clip? behavior, {
  bool paintRedFirst = false,
}) async {
  final key = GlobalKey();
  final contents = Stack(
    fit: StackFit.expand,
    children: [
      if (paintRedFirst) const ColoredBox(color: Color(0xffff0000)),
      const ColoredBox(color: Color(0xffffffff)),
    ],
  );
  const radius = BorderRadius.all(Radius.circular(12));
  final clip = behavior == null
      ? ClipRRect(borderRadius: radius, child: contents)
      : ClipRRect(
          borderRadius: radius,
          clipBehavior: behavior,
          child: contents,
        );
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: RepaintBoundary(
          key: key,
          child: SizedBox.square(
            dimension: 48,
            child: Stack(
              fit: StackFit.expand,
              children: [
                const ColoredBox(color: Color(0xff000000)),
                Positioned(left: 4, top: 4, width: 40, height: 40, child: clip),
              ],
            ),
          ),
        ),
      ),
    ),
  );

  final boundary = tester.renderObject<RenderRepaintBoundary>(find.byKey(key));
  final result = await tester.runAsync(() async {
    final image = await boundary.toImage(pixelRatio: 1);
    try {
      expect((image.width, image.height), (48, 48));
      final rgba = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      List<int> pixel(int x, int y) =>
          List.generate(4, (c) => rgba.getUint8((y * image.width + x) * 4 + c));
      final cornerPixels = [
        for (var y = 4; y < 16; y++)
          for (var x = 4; x < 16; x++) pixel(x, y),
      ];
      return (
        corner: pixel(4, 4),
        center: pixel(24, 24),
        partial: cornerPixels.where((p) => p[1] > 0 && p[1] < 255).length,
        redFringe: cornerPixels.where((p) => p[0] > p[1] + 1).length,
      );
    } finally {
      image.dispose();
    }
  });
  return result!;
}

void main() {
  testWidgets('ClipRRect default smooths an edge that hardEdge does not', (
    tester,
  ) async {
    final none = await _capture(tester, Clip.none);
    expect(none.corner, _white);
    expect(none.center, _white);
    final hard = await _capture(tester, Clip.hardEdge);
    final defaultClip = await _capture(tester, null);
    for (final probe in [hard, defaultClip]) {
      expect(probe.corner, _black);
      expect(probe.center, _white);
    }
    expect(hard.partial, 0);
    expect(defaultClip.partial, greaterThan(0));
  });

  testWidgets('ClipRRect saveLayer removes earlier paint from the AA edge', (
    tester,
  ) async {
    final ordinary = await _capture(tester, null, paintRedFirst: true);
    final isolated = await _capture(
      tester,
      Clip.antiAliasWithSaveLayer,
      paintRedFirst: true,
    );
    for (final probe in [ordinary, isolated]) {
      expect(probe.corner, _black);
      expect(probe.center, _white);
      expect(probe.partial, greaterThan(0));
    }
    expect(ordinary.redFringe, greaterThan(0));
    expect(isolated.redFringe, 0);
  });
}
