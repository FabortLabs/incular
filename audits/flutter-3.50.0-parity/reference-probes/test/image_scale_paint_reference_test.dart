import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class _ImageCanvas implements ui.Canvas {
  final draws = <(ui.Rect, ui.Rect)>[];

  @override
  void drawImageRect(ui.Image image, ui.Rect src, ui.Rect dst, ui.Paint paint) {
    draws.add((src, dst));
  }

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
}
