import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _black = [0, 0, 0, 255];
const _white = [255, 255, 255, 255];

Future<({List<List<int>> controls, int partial})> _rawClip(
  double factor, {
  bool? pathClip,
  bool defaultRounded = false,
}) async {
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  canvas.drawColor(const Color(0xff000000), ui.BlendMode.src);
  canvas.translate(4, 4);
  canvas.scale(factor);
  final size = 40.0 / factor;
  final rect = ui.Rect.fromLTWH(0, 0, size, size);
  if (pathClip == true) {
    final edge = 12.0 / factor;
    final path = ui.Path()
      ..moveTo(edge, 0)
      ..lineTo(size, 0)
      ..lineTo(size, size)
      ..lineTo(0, size)
      ..lineTo(0, edge)
      ..close();
    canvas.clipPath(path, doAntiAlias: false);
  } else if (pathClip == false) {
    final rounded = ui.RRect.fromRectAndRadius(
      rect,
      ui.Radius.circular(12 / factor),
    );
    if (defaultRounded) {
      canvas.clipRRect(rounded);
    } else {
      canvas.clipRRect(rounded, doAntiAlias: false);
    }
  }
  canvas.drawRect(rect, ui.Paint()..color = const Color(0xffffffff));
  final picture = recorder.endRecording();
  try {
    final image = await picture.toImage(48, 48);
    try {
      final bytes = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      final controls = [(4, 4), (16, 16), (34, 16), (45, 16)].map((point) {
        final index = (point.$2 * 48 + point.$1) * 4;
        return List.generate(4, (channel) => bytes.getUint8(index + channel));
      }).toList();
      var partial = 0;
      for (var y = 4; y < 16; y++) {
        for (var x = 4; x < 16; x++) {
          final g = bytes.getUint8((y * 48 + x) * 4 + 1);
          if (g > 0 && g < 255) partial++;
        }
      }
      return (controls: controls, partial: partial);
    } finally {
      image.dispose();
    }
  } finally {
    picture.dispose();
  }
}

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
  test('Hard Canvas clips follow paint transforms', () async {
    for (final pathClip in [false, true]) {
      for (final factor in [1.0, 2.0]) {
        final probe = await _rawClip(factor, pathClip: pathClip);
        expect(probe.controls, [
          _black,
          _white,
          _white,
          _black,
        ], reason: 'pathClip=$pathClip, factor=$factor');
        expect(probe.partial, 0);
      }
    }
    expect((await _rawClip(2)).controls, [_white, _white, _white, _black]);
  });
  test('Default rounded Canvas coverage survives scaling', () async {
    final unscaled = await _rawClip(1, pathClip: false, defaultRounded: true);
    final scaled = await _rawClip(2, pathClip: false, defaultRounded: true);
    expect(unscaled.controls, [_black, _white, _white, _black]);
    expect(scaled.controls, unscaled.controls);
    expect(unscaled.partial, greaterThan(0));
    expect(scaled.partial, unscaled.partial);
  });
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
