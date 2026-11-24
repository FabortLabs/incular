import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _red = [255, 0, 0, 255];
const _blue = [0, 0, 255, 255];
const _black = [0, 0, 0, 255];
const _original = [_red, _red, _red, _blue, _blue, _black];
const _originalProbes = [8, 10, 11, 12, 15, 16];
const _enlargedProbes = [12, 15, 16, 19, 25];

ui.ImageFilter _matrix(double scale, {bool nearest = false}) {
  final values = Float64List.fromList([
    scale, 0, 0, 0, //
    0, scale, 0, 0, //
    0, 0, 1, 0, //
    0, 0, 0, 1, //
  ]);
  return nearest
      ? ui.ImageFilter.matrix(values, filterQuality: ui.FilterQuality.none)
      : ui.ImageFilter.matrix(values);
}

Future<void> _check(
  WidgetTester tester,
  ui.ImageFilter imageFilter,
  List<int> probes,
  List<List<int>> expected, {
  bool enabled = true,
}) async {
  final key = GlobalKey();
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: RepaintBoundary(
          key: key,
          child: SizedBox(
            width: 40,
            height: 32,
            child: Stack(
              fit: StackFit.expand,
              children: [
                const ColoredBox(color: Color(0xff000000)),
                Positioned(
                  left: 8,
                  top: 8,
                  child: ImageFiltered(
                    enabled: enabled,
                    imageFilter: imageFilter,
                    child: const Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        ColoredBox(
                          color: Color(0xffff0000),
                          child: SizedBox(width: 4, height: 8),
                        ),
                        ColoredBox(
                          color: Color(0xff0000ff),
                          child: SizedBox(width: 4, height: 8),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    ),
  );
  final boundary = tester.renderObject<RenderRepaintBoundary>(find.byKey(key));
  await tester.runAsync(() async {
    final image = await boundary.toImage(pixelRatio: 1);
    try {
      expect((image.width, image.height), (40, 32));
      final rgba = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      final actual = [
        for (final x in probes)
          List.generate(
            4,
            (c) => rgba.getUint8((12 * image.width + x) * 4 + c),
          ),
      ];
      for (var i = 0; i < probes.length; i++) {
        for (var c = 0; c < 4; c++) {
          expect(
            actual[i][c],
            closeTo(expected[i][c], 1),
            reason: 'x=${probes[i]}, samples=$actual',
          );
        }
      }
    } finally {
      image.dispose();
    }
  });
}

void main() {
  testWidgets(
    'ImageFiltered identity and disabled matrix retain original pixels',
    (tester) async {
      await _check(tester, _matrix(1), _originalProbes, _original);
      await _check(
        tester,
        _matrix(2),
        _originalProbes,
        _original,
        enabled: false,
      );
    },
  );
  testWidgets(
    'ImageFiltered default matrix sampling blends the enlarged tile seam',
    (tester) async {
      await _check(tester, _matrix(2, nearest: true), _enlargedProbes, [
        _red,
        _red,
        _blue,
        _blue,
        _black,
      ]);
      await _check(tester, _matrix(2), _enlargedProbes, [
        _red,
        [191, 0, 64, 255],
        [64, 0, 191, 255],
        _blue,
        _black,
      ]);
    },
  );
}
