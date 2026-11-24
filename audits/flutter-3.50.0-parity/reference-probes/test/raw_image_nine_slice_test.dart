import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _width = 100;
const _height = 60;
const _centerSlice = ui.Rect.fromLTWH(10, 10, 10, 10);
const _centered = [35, 15, 65, 45];
const _fillsOutput = [0, 0, _width, _height];

Future<List<int>> _bounds(
  WidgetTester tester,
  ui.Image source, {
  ui.Rect? centerSlice,
  BoxFit? fit,
}) async {
  final boundary = GlobalKey();
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: RepaintBoundary(
          key: boundary,
          child: ColoredBox(
            color: const ui.Color(0xff000000),
            child: RawImage(
              image: source,
              width: _width.toDouble(),
              height: _height.toDouble(),
              centerSlice: centerSlice,
              fit: fit,
              filterQuality: ui.FilterQuality.none,
            ),
          ),
        ),
      ),
    ),
  );
  final render = tester.renderObject<RenderRepaintBoundary>(
    find.byKey(boundary),
  );
  return (await tester.runAsync(() async {
    final rendered = await render.toImage(pixelRatio: 1);
    try {
      expect((rendered.width, rendered.height), (_width, _height));
      final bytes = (await rendered.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      var left = _width, top = _height, right = 0, bottom = 0, painted = 0;
      for (var y = 0; y < _height; y++) {
        for (var x = 0; x < _width; x++) {
          if (bytes.getUint8((y * _width + x) * 4) == 0) {
            continue;
          }
          painted++;
          if (x < left) left = x;
          if (y < top) top = y;
          if (x + 1 > right) right = x + 1;
          if (y + 1 > bottom) bottom = y + 1;
        }
      }
      expect(painted, greaterThan(0));
      expect(
        painted,
        (right - left) * (bottom - top),
        reason: 'no holes in the painted bounds',
      );
      return [left, top, right, bottom];
    } finally {
      rendered.dispose();
    }
  }))!;
}

void main() {
  testWidgets('RawImage defaults to fill only when centerSlice is set', (
    tester,
  ) async {
    final recorder = ui.PictureRecorder();
    ui.Canvas(recorder).drawColor(const ui.Color(0xffffffff), ui.BlendMode.src);
    final picture = recorder.endRecording();
    final source = picture.toImageSync(30, 30);
    picture.dispose();
    addTearDown(source.dispose);

    expect(await _bounds(tester, source), _centered, reason: 'no slice');
    expect(
      await _bounds(
        tester,
        source,
        centerSlice: _centerSlice,
        fit: BoxFit.scaleDown,
      ),
      _centered,
      reason: 'explicit scaleDown',
    );
    expect(
      await _bounds(
        tester,
        source,
        centerSlice: _centerSlice,
        fit: BoxFit.fill,
      ),
      _fillsOutput,
      reason: 'explicit fill',
    );
    expect(
      await _bounds(tester, source, centerSlice: _centerSlice),
      _fillsOutput,
      reason: 'unset fit with slice',
    );
  });
}
