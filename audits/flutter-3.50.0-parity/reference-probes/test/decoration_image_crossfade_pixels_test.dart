import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class PixelProvider extends ImageProvider<PixelProvider> {
  const PixelProvider(this.pixel);

  final ui.Image pixel;

  @override
  Future<PixelProvider> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(PixelProvider key, ImageDecoderCallback decode) =>
      OneFrameImageStreamCompleter(SynchronousFuture(ImageInfo(image: pixel.clone())));
}

ui.Image solid(Color color) {
  final recorder = ui.PictureRecorder();
  ui.Canvas(recorder).drawColor(color, BlendMode.src);
  final picture = recorder.endRecording();
  final image = picture.toImageSync(1, 1);
  picture.dispose();
  return image;
}

void main() {
  testWidgets('two half-opacity decorations blend before reaching the colored backdrop', (
    WidgetTester tester,
  ) async {
    final red = solid(const Color(0xFFFF0000));
    final green = solid(const Color(0xFF00FF00));
    final a = DecorationImage(image: PixelProvider(red), fit: BoxFit.fill, opacity: 0.5);
    final b = DecorationImage(image: PixelProvider(green), fit: BoxFit.fill, opacity: 0.5);
    final canvas = GlobalKey();
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: RepaintBoundary(
            key: canvas,
            child: ColoredBox(
              color: const Color(0xFF0000FF),
              child: SizedBox.square(
                dimension: 16,
                child: DecoratedBox(decoration: BoxDecoration(image: DecorationImage.lerp(a, b, 0.5))),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    final boundary = canvas.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final rgba = (await tester.runAsync(() async {
      final image = boundary.toImageSync();
      final bytes = (await image.toByteData())!.buffer.asUint8List();
      image.dispose();
      return bytes;
    }))!;
    final offset = (8 * 16 + 8) * 4;
    expect(rgba.sublist(offset, offset + 4), [64, 64, 127, 255]);
    red.dispose();
    green.dispose();
  });
}
