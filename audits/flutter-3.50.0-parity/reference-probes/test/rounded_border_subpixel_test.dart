import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

Future<List<int>> edgeCoverage(double width, BorderStyle style) async {
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  Border.fromBorderSide(
    BorderSide(color: const Color(0xff000000), width: width, style: style),
  ).paint(
    canvas,
    const Rect.fromLTWH(16.25, 16.25, 32, 32),
    borderRadius: BorderRadius.circular(8),
  );
  final picture = recorder.endRecording();
  final image = await picture.toImage(64, 64);
  final rgba = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  final edge = [
    for (final x in [15, 16, 17, 18, 19, 32])
      rgba.getUint8((32 * 64 + x) * 4 + 3),
    for (final (x, y) in [
      (18, 18),
      (17, 18),
      (19, 18),
      (18, 17),
      (18, 19),
      (19, 19),
    ])
      rgba.getUint8((y * 64 + x) * 4 + 3),
  ];
  image.dispose();
  picture.dispose();
  return edge;
}

void main() {
  test('a fractional rounded border does not leave a phantom stronger stroke', () async {
    final thin = await edgeCoverage(0.03125, BorderStyle.solid);
    final thick = await edgeCoverage(2, BorderStyle.solid);
    final omitted = await edgeCoverage(0.03125, BorderStyle.none);
    final tiny = await edgeCoverage(0.001, BorderStyle.solid);
    final filled = await edgeCoverage(16, BorderStyle.solid);
    // Report the observed samples for the renderer comparison.
    print(
      'rounded border alpha thin=$thin thick=$thick none=$omitted tiny=$tiny filled=$filled',
    );
    expect(omitted, everyElement(0));
    expect(thin[1], inInclusiveRange(1, 16));
    expect(thin.sublist(2, 6), everyElement(0));
    expect(thin.sublist(6), everyElement(inInclusiveRange(0, 16)));
    expect(tiny, everyElement(inInclusiveRange(0, 1)));
    expect(thick[1], greaterThan(128));
    expect(thick[2], 255);
    expect(thick[5], 0);
    expect(filled[5], 255);
  });
}
