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

ui.Image solid(Color color, [int size = 1]) {
  final recorder = ui.PictureRecorder();
  ui.Canvas(recorder).drawColor(color, BlendMode.src);
  final picture = recorder.endRecording();
  final image = picture.toImageSync(size, size);
  picture.dispose();
  return image;
}

Future<List<int>> paintedCenter(
  WidgetTester tester,
  DecorationImage image, {
  int x = 8,
  int y = 8,
}) async {
  final key = GlobalKey();
  await tester.pumpWidget(
    Center(
      child: RepaintBoundary(
        key: key,
        child: ColoredBox(
          color: const Color(0xFF0000FF),
          child: SizedBox.square(
            dimension: 16,
            child: DecoratedBox(decoration: BoxDecoration(image: image)),
          ),
        ),
      ),
    ),
  );
  final target = key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  return (await tester.runAsync(() async {
    final output = target.toImageSync();
    final pixels = (await output.toByteData())!.buffer.asUint8List();
    output.dispose();
    final index = (y * 16 + x) * 4;
    return pixels.sublist(index, index + 4);
  }))!;
}

void main() {
  testWidgets('decoration image inversion applies after its color filter', (tester) async {
    final bitmap = solid(const Color(0xFFFF0000));
    addTearDown(bitmap.dispose);
    expect(await paintedCenter(tester, DecorationImage(
      image: PixelProvider(bitmap),
      fit: BoxFit.fill,
      invertColors: true,
      colorFilter: const ColorFilter.mode(Color(0xFF00FF00), BlendMode.srcIn),
    )), [255, 0, 255, 255]);
  });

  testWidgets('decoration opacity multiplies the filtered alpha once', (tester) async {
    final bitmap = solid(const Color(0xFFFF0000));
    addTearDown(bitmap.dispose);
    expect(await paintedCenter(tester, DecorationImage(
      image: PixelProvider(bitmap),
      fit: BoxFit.fill,
      opacity: 0.5,
      colorFilter: const ColorFilter.mode(Color(0x8000FF00), BlendMode.srcIn),
    )), [0, 64, 191, 255]);
  });

  testWidgets('an unfiltered inverted decoration keeps the backdrop untouched', (tester) async {
    final bitmap = solid(const Color(0xFFFF0000));
    addTearDown(bitmap.dispose);
    expect(await paintedCenter(tester, DecorationImage(
      image: PixelProvider(bitmap),
      fit: BoxFit.fill,
      invertColors: true,
    )), [0, 255, 255, 255]);
  });

  testWidgets('a source color filter does not paint outside the image', (tester) async {
    final bitmap = solid(const Color(0xFFFF0000), 4);
    addTearDown(bitmap.dispose);
    final decoration = DecorationImage(
      image: PixelProvider(bitmap),
      fit: BoxFit.none,
      colorFilter: const ColorFilter.mode(Color(0xFF00FF00), BlendMode.src),
    );
    expect(await paintedCenter(tester, decoration), [0, 255, 0, 255]);
    expect(await paintedCenter(tester, decoration, x: 1, y: 1), [0, 0, 255, 255]);
  });

}
