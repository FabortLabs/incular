import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class SizedProvider extends ImageProvider<(SizedProvider, int)> {
  final pending = <int, Completer<ImageInfo>>{};

  @override
  Future<(SizedProvider, int)> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture((this, configuration.size!.width.round()));

  @override
  ImageStreamCompleter loadImage((SizedProvider, int) key, ImageDecoderCallback decode) {
    return OneFrameImageStreamCompleter(pending.putIfAbsent(key.$2, Completer<ImageInfo>.new).future);
  }

  void finish(int width, ui.Color color) {
    pending[width]!.complete(ImageInfo(image: solid(color)));
  }
}

class Placeholder extends ImageProvider<Placeholder> {
  const Placeholder();

  @override
  Future<Placeholder> obtainKey(ImageConfiguration configuration) => SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(Placeholder key, ImageDecoderCallback decode) =>
      OneFrameImageStreamCompleter(SynchronousFuture(ImageInfo(image: solid(const ui.Color(0xFF00FF00)))));
}

ui.Image solid(ui.Color color) {
  final recorder = ui.PictureRecorder();
  ui.Canvas(recorder).drawColor(color, ui.BlendMode.src);
  final picture = recorder.endRecording();
  final result = picture.toImageSync(1, 1);
  picture.dispose();
  return result;
}

Future<List<int>> sample(WidgetTester tester, GlobalKey key, int x) async {
  final target = key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  return (await tester.runAsync(() async {
    final result = target.toImageSync();
    final pixels = (await result.toByteData())!.buffer.asUint8List();
    final index = (result.width * 8 + x) * 4;
    result.dispose();
    return pixels.sublist(index, index + 4);
  }))!;
}

void main() {
  for (final useFallback in [true, false]) {
    testWidgets('a resize keeps its last primary until the new size loads, fallback $useFallback', (tester) async {
      final provider = SizedProvider();
      final key = GlobalKey();
      final decoration = BoxDecoration(
        color: const ui.Color(0xFF000000),
        image: DecorationImage(image: provider, placeholder: useFallback ? const Placeholder() : null, fit: BoxFit.fill),
      );
      Future<void> resize(double width) => tester.pumpWidget(
        Center(
          child: RepaintBoundary(
            key: key,
            child: SizedBox(
              width: width,
              height: 16,
              child: DecoratedBox(decoration: decoration),
            ),
          ),
        ),
      );

      await resize(40);
      expect(await sample(tester, key, 30), useFallback ? [0, 255, 0, 255] : [0, 0, 0, 255]);
      provider.finish(40, const ui.Color(0xFFFF0000));
      await tester.pump();
      await tester.pump();
      expect(await sample(tester, key, 30), [255, 0, 0, 255]);

      await resize(80);
      expect(provider.pending.keys, [40, 80]);
      expect(await sample(tester, key, 70), [255, 0, 0, 255]);
      provider.finish(80, const ui.Color(0xFF0000FF));
      await tester.pump();
      await tester.pump();
      expect(await sample(tester, key, 70), [0, 0, 255, 255]);
    });
  }
}
