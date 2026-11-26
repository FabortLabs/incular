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
  TextDirection? direction,
  bool logicalAlignment = false,
  bool mirror = true,
}) async {
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  if (direction == null) {
    canvas.drawColor(const ui.Color(0xffffffff), ui.BlendMode.src);
  } else {
    canvas.drawRect(
      const ui.Rect.fromLTWH(0, 0, 1, 2),
      ui.Paint()..color = const ui.Color(0xffff0000),
    );
    canvas.drawRect(
      const ui.Rect.fromLTWH(1, 0, 1, 2),
      ui.Paint()..color = const ui.Color(0xff0000ff),
    );
  }
  final picture = recorder.endRecording();
  final source = picture.toImageSync(2, 2);
  picture.dispose();
  addTearDown(source.dispose);
  final boundary = GlobalKey();
  await tester.pumpWidget(
    Directionality(
      textDirection: direction ?? TextDirection.ltr,
      child: Center(
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
                alignment: logicalAlignment
                    ? AlignmentDirectional.topStart
                    : (direction == null
                          ? Alignment.center
                          : Alignment.topLeft),
                matchTextDirection: direction != null && mirror,
                filterQuality: ui.FilterQuality.none,
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

      if (logicalAlignment && direction == TextDirection.rtl) {
        return (pixel(4, 0), pixel(3, 0));
      }
      return direction == null
          ? (pixel(3, 2), pixel(0, 2))
          : (pixel(0, 0), pixel(2, 0));
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

  testWidgets(
    'RawImage mirrors source pixels without moving physical alignment or widening Src',
    (tester) async {
      for (final (direction, inside) in [
        (TextDirection.ltr, (255, 0, 0, 255)),
        (TextDirection.rtl, (0, 0, 255, 255)),
      ]) {
        expect(
          await _sample(
            tester,
            blend: ui.BlendMode.src,
            background: const ui.Color(0xff00ff00),
            direction: direction,
          ),
          (inside, (0, 255, 0, 255)),
        );
      }
    },
  );

  testWidgets('RawImage matchTextDirection requires Directionality', (
    tester,
  ) async {
    await tester.pumpWidget(const RawImage(matchTextDirection: true));
    expect(
      tester.takeException().toString(),
      contains('No Directionality widget found'),
    );
  });

  testWidgets(
    'RawImage logical alignment resolves independently of mirroring',
    (tester) async {
      for (final mirror in [false, true]) {
        for (final direction in [TextDirection.ltr, TextDirection.rtl]) {
          final pixel = mirror && direction == TextDirection.rtl
              ? (0, 0, 255, 255)
              : (255, 0, 0, 255);
          expect(
            await _sample(
              tester,
              blend: ui.BlendMode.src,
              background: const ui.Color(0xff00ff00),
              direction: direction,
              logicalAlignment: true,
              mirror: mirror,
            ),
            (pixel, (0, 255, 0, 255)),
          );
        }
      }
    },
  );
}
