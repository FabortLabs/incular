import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _black = Color(0xff000000);
const _opaque = Color(0xff406080);
const _half = Color(0x80406080);
const _width = 64;
const _height = 48;
const _blackPixel = [0, 0, 0, 255];
const _matte = [0, 32, 0, 255];
const _filtered = [128, 128, 64, 255];
const _filteredHalf = [64, 64, 32, 255];
const _rgbRows = <double>[
  0, 0, 1, 0, 0, // R' = B.
  0, 1, 0, 0, 32, // G' = G + 32.
  1, 0, 0, 0, 0, // B' = R.
];
const _matrix = ui.ColorFilter.matrix([
  ..._rgbRows,
  0, 0, 0, 0, 255, // A' = 255.
]);
const _preserveAlpha = ui.ColorFilter.matrix([
  ..._rgbRows,
  0, 0, 0, 1, 0, // A' = A.
]);
const _halfAlpha = ui.ColorFilter.matrix([
  ..._rgbRows,
  0, 0, 0, 0, 128, // A' = 128.
]);

Widget _box(Color color) =>
    ColoredBox(color: color, child: const SizedBox(width: 32, height: 24));

Widget _filter(Widget child, [ui.ColorFilter matrix = _matrix]) =>
    ColorFiltered(colorFilter: matrix, child: child);

Future<void> _check(
  WidgetTester tester,
  String label,
  Widget child,
  List<int> foreground, {
  List<int> backdrop = _blackPixel,
  int clipPadding = 0,
}) async {
  final key = GlobalKey();
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: RepaintBoundary(
          key: key,
          child: SizedBox(
            width: _width.toDouble(),
            height: _height.toDouble(),
            child: Stack(
              fit: StackFit.expand,
              children: [
                const ColoredBox(color: _black),
                Positioned(left: 8, top: 12, child: child),
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
      expect((image.width, image.height), (_width, _height));
      final bytes = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      for (var y = 0; y < _height; y++) {
        for (var x = 0; x < _width; x++) {
          final inside =
              x >= 8 + clipPadding &&
              x < 40 + clipPadding &&
              y >= 12 + clipPadding &&
              y < 36 + clipPadding;
          final outsideClip =
              clipPadding != 0 &&
              (x < 8 ||
                  x >= 40 + 2 * clipPadding ||
                  y < 12 ||
                  y >= 36 + 2 * clipPadding);
          final expected = inside
              ? foreground
              : outsideClip
              ? _blackPixel
              : backdrop;
          final index = (y * _width + x) * 4;
          final actual = List.generate(4, (c) => bytes.getUint8(index + c));
          if (List.generate(
            4,
            (c) => (actual[c] - expected[c]).abs(),
          ).any((d) => d > 1)) {
            fail('$label pixel ($x,$y) = $actual, expected $expected');
          }
        }
      }
    } finally {
      image.dispose();
    }
  });
}

void main() {
  testWidgets(
    'ColorFiltered matrix recovers straight RGB before replacing alpha',
    (tester) async {
      await _check(tester, 'unfiltered translucent', _box(_half), [
        32,
        48,
        64,
        255,
      ]);
      await _check(
        tester,
        'alpha-preserving matrix',
        _filter(_box(_half), _preserveAlpha),
        _filteredHalf,
      );
      await _check(
        tester,
        'filtered opaque',
        _filter(_box(_opaque)),
        _filtered,
        backdrop: _matte,
      );
      await _check(
        tester,
        'filtered translucent',
        _filter(_box(_half)),
        _filtered,
        backdrop: _matte,
      );
    },
  );

  testWidgets('ColorFiltered and Opacity respect their nesting order', (
    tester,
  ) async {
    await _check(
      tester,
      'filter then opacity',
      Opacity(opacity: 0.5, child: _filter(_box(_opaque))),
      _filteredHalf,
    );
    await _check(
      tester,
      'opacity then filter',
      _filter(Opacity(opacity: 0.5, child: _box(_opaque))),
      _filtered,
      backdrop: _matte,
    );
  });

  testWidgets(
    'an explicit clip or zero-size child limits a ColorFiltered layer',
    (tester) async {
      await _check(
        tester,
        'clipped filter',
        ClipRect(child: _filter(_box(_opaque))),
        _filtered,
      );
      await _check(
        tester,
        'clip larger than translucent matte',
        ClipRect(
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: _filter(_box(_opaque), _halfAlpha),
          ),
        ),
        _filteredHalf,
        backdrop: [0, 16, 0, 255],
        clipPadding: 4,
      );
      await _check(
        tester,
        'empty filter',
        _filter(const SizedBox.shrink()),
        _blackPixel,
      );
    },
  );
}
