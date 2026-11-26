import 'dart:ui' as ui;

import 'package:flutter/animation.dart';
import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

typedef Pixel = (int, int, int, int);

Future<(Pixel, Pixel)> _sample(
  WidgetTester tester, {
  required ui.BlendMode blend,
  required ui.Color background,
  ui.Color? tint,
  bool invert = false,
  double opacity = 1,
}) async {
  final recorder = ui.PictureRecorder();
  ui.Canvas(recorder).drawColor(const ui.Color(0xffffffff), ui.BlendMode.src);
  final picture = recorder.endRecording();
  final source = picture.toImageSync(2, 2);
  picture.dispose();
  addTearDown(source.dispose);
  final boundary = GlobalKey();
  await tester.pumpWidget(
    Center(
      child: RepaintBoundary(
        key: boundary,
        child: ColoredBox(
          color: background,
          child: SizedBox(
            width: 6,
            height: 4,
            child: RawImage(
              image: source,
              blendMode: blend,
              color: tint,
              invertColors: invert,
              opacity: AlwaysStoppedAnimation<double>(opacity),
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
    final image = await render.toImage();
    try {
      final bytes = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      Pixel pixel(int x, int y) {
        final offset = (y * image.width + x) * 4;
        return (
          bytes.getUint8(offset),
          bytes.getUint8(offset + 1),
          bytes.getUint8(offset + 2),
          bytes.getUint8(offset + 3),
        );
      }

      return (pixel(3, 2), pixel(0, 2));
    } finally {
      image.dispose();
    }
  }))!;
}

void main() {
  testWidgets('RawImage applies inversion after tint and before blending', (
    tester,
  ) async {
    expect(
      await _sample(
        tester,
        blend: ui.BlendMode.src,
        background: const ui.Color(0xff00ff00),
        tint: const ui.Color(0xffff0000),
        invert: true,
      ),
      ((0, 255, 255, 255), (0, 255, 0, 255)),
    );
  });

  testWidgets('RawImage zero opacity still replaces fitted pixels under Src', (
    tester,
  ) async {
    expect(
      await _sample(
        tester,
        blend: ui.BlendMode.src,
        background: const ui.Color(0xffff0000),
        opacity: 0,
      ),
      ((0, 0, 0, 0), (255, 0, 0, 255)),
    );
    expect(
      await _sample(
        tester,
        blend: ui.BlendMode.srcOver,
        background: const ui.Color(0xffff0000),
        opacity: 0,
      ),
      ((255, 0, 0, 255), (255, 0, 0, 255)),
    );
  });
}
