import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';

Future<({int partial, List<int> samples})> _raster({
  required bool antialias,
}) async {
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder)
    ..scale(1.25)
    ..translate(24, 24)
    ..rotate(0.65)
    ..scale(1.1, 0.85)
    ..clipPath(
      ui.Path()..addOval(const ui.Rect.fromLTWH(-18, -8, 36, 16)),
      doAntiAlias: antialias,
    )
    ..drawRect(
      const ui.Rect.fromLTWH(-25, -25, 50, 50),
      ui.Paint()..color = const ui.Color(0xffffffff),
    );
  final picture = recorder.endRecording();
  try {
    final image = await picture.toImage(64, 64);
    try {
      final raw = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
      int alpha(int x, int y) => raw.getUint8((y * 64 + x) * 4 + 3);
      final samples = <int>[
        for (final (x, y) in [(0.0, 0.0), (10.0, 2.0), (8.0, 11.0)])
          alpha(
            ((24 + math.cos(0.65) * x * 1.1 - math.sin(0.65) * y * 0.85) * 1.25)
                .floor(),
            ((24 + math.sin(0.65) * x * 1.1 + math.cos(0.65) * y * 0.85) * 1.25)
                .floor(),
          ),
      ];
      var partial = 0;
      for (var y = 0; y < 64; y++) {
        for (var x = 0; x < 64; x++) {
          final a = alpha(x, y);
          if (a != 0 && a != 255) {
            partial++;
          }
        }
      }
      return (partial: partial, samples: samples);
    } finally {
      image.dispose();
    }
  } finally {
    picture.dispose();
  }
}

void main() {
  testWidgets(
    'a transformed oval keeps partial coverage only with antialiasing',
    (tester) async {
      final outputs = await tester.runAsync(() async {
        final soft = await _raster(antialias: true);
        final hard = await _raster(antialias: false);
        return (soft: soft, hard: hard);
      });
      expect(outputs!.soft.samples, [255, 255, 0]);
      expect(outputs.hard.samples, [255, 255, 0]);
      expect(outputs.soft.partial, greaterThanOrEqualTo(30));
      expect(outputs.hard.partial, 0);
    },
  );
}
