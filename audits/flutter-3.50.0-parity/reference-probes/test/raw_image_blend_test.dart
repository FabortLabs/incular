import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('RawImage blends only inside its fitted destination', (
    WidgetTester tester,
  ) async {
    final result = await tester.runAsync(() async {
      final sourceRecorder = ui.PictureRecorder();
      final sourceCanvas = Canvas(sourceRecorder);
      sourceCanvas.drawColor(const Color(0xFF00FF00), BlendMode.src);
      final sourcePicture = sourceRecorder.endRecording();
      final image = await sourcePicture.toImage(10, 10);
      sourcePicture.dispose();
      final boundary = GlobalKey();
      try {
        await tester.pumpWidget(
          RepaintBoundary(
            key: boundary,
            child: Center(
              child: ColoredBox(
                color: const Color(0xFFFF0000),
                child: SizedBox(
                  width: 40,
                  height: 20,
                  child: RawImage(image: image, blendMode: BlendMode.src),
                ),
              ),
            ),
          ),
        );
        final render =
            boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary;
        final screenshot = await render.toImage();
        try {
          final bytes = (await screenshot.toByteData())!;
          List<int> pixel(int x, int y) => List<int>.generate(
            4,
            (int index) =>
                bytes.getUint8((y * screenshot.width + x) * 4 + index),
          );
          final x = screenshot.width ~/ 2;
          final y = screenshot.height ~/ 2;
          return <List<int>>[
            pixel(x, y),
            pixel(x - 5, y),
            pixel(x - 6, y),
            pixel(x + 5, y),
          ];
        } finally {
          screenshot.dispose();
        }
      } finally {
        await tester.pumpWidget(const SizedBox());
        image.dispose();
      }
    });
    expect(result, <List<int>>[
      <int>[0, 255, 0, 255],
      <int>[0, 255, 0, 255],
      <int>[255, 0, 0, 255],
      <int>[255, 0, 0, 255],
    ]);
  });
}
