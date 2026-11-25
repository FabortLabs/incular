import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class _ImageCanvas implements ui.Canvas {
  final draws = <(ui.Rect, ui.Rect)>[];
  final clips = <ui.Rect>[];
  final drawClipDepth = <int>[];
  final _savedClipDepths = <int>[];
  int _clipDepth = 0;

  @override
  void drawImageRect(ui.Image image, ui.Rect src, ui.Rect dst, ui.Paint paint) {
    draws.add((src, dst));
    drawClipDepth.add(_clipDepth);
  }

  @override
  void save() => _savedClipDepths.add(_clipDepth);

  @override
  void clipRect(
    ui.Rect rect, {
    ui.ClipOp clipOp = ui.ClipOp.intersect,
    bool doAntiAlias = true,
  }) {
    clips.add(rect);
    _clipDepth++;
  }

  @override
  void restore() => _clipDepth = _savedClipDepths.removeLast();

  @override
  void noSuchMethod(Invocation invocation) {}
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('scaled image paints pixels into the expected logical box', () async {
    final image = await createTestImage(width: 40, height: 20, cache: false);
    addTearDown(image.dispose);

    final first = _ImageCanvas();
    paintImage(
      canvas: first,
      rect: const ui.Rect.fromLTWH(0, 0, 100, 100),
      image: image,
      scale: 2,
    );
    expect(first.draws, [
      (
        const ui.Rect.fromLTWH(0, 0, 40, 20),
        const ui.Rect.fromLTWH(40, 45, 20, 10),
      ),
    ]);

    final second = _ImageCanvas();
    paintImage(
      canvas: second,
      rect: const ui.Rect.fromLTWH(0, 0, 10, 20),
      image: image,
      scale: 2,
      fit: BoxFit.none,
    );
    expect(second.draws, [
      (
        const ui.Rect.fromLTWH(10, 0, 20, 20),
        const ui.Rect.fromLTWH(0, 5, 10, 10),
      ),
    ]);
  });

  test(
    'repeating an image clips only when tiles extend beyond its box',
    () async {
      final image = await createTestImage(width: 20, height: 20, cache: false);
      addTearDown(image.dispose);
      for (final (size, repeat, xs, ys) in [
        (50.0, ImageRepeat.repeat, [-5.0, 15.0, 35.0], [-5.0, 15.0, 35.0]),
        (50.0, ImageRepeat.repeatX, [-5.0, 15.0, 35.0], [15.0]),
        (50.0, ImageRepeat.repeatY, [15.0], [-5.0, 15.0, 35.0]),
        (20.0, ImageRepeat.repeat, [0.0], [0.0]),
      ]) {
        final canvas = _ImageCanvas();
        final bounds = ui.Rect.fromLTWH(0, 0, size, size);
        paintImage(
          canvas: canvas,
          rect: bounds,
          image: image,
          fit: BoxFit.none,
          repeat: repeat,
        );
        final repeated = size != 20;
        expect(canvas.clips, repeated ? [bounds] : isEmpty);
        expect(
          canvas.drawClipDepth,
          List.filled(xs.length * ys.length, repeated ? 1 : 0),
        );
        expect(
          canvas.draws,
          unorderedEquals([
            for (final x in xs)
              for (final y in ys)
                (
                  const ui.Rect.fromLTWH(0, 0, 20, 20),
                  ui.Rect.fromLTWH(x, y, 20, 20),
                ),
          ]),
        );
      }
    },
  );
}
