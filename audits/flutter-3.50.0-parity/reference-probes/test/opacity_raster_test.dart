import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _width = 100;
const _height = 80;
const _blue = Color(0xff0000ff);
const _black = Color(0xff000000);

Future<void> _mount(WidgetTester tester, GlobalKey key, Widget child) =>
    tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: RepaintBoundary(
            key: key,
            child: SizedBox(
              width: _width.toDouble(),
              height: _height.toDouble(),
              child: Stack(
                fit: StackFit.expand,
                children: [
                  const ColoredBox(color: _black),
                  child,
                ],
              ),
            ),
          ),
        ),
      ),
    );

Future<Uint8List> _read(WidgetTester tester, GlobalKey key) async {
  final boundary = tester.renderObject<RenderRepaintBoundary>(find.byKey(key));
  return (await tester.runAsync(() async {
    final image = await boundary.toImage(pixelRatio: 1);
    try {
      expect((image.width, image.height), (_width, _height));
      final bytes = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      return bytes.buffer.asUint8List(bytes.offsetInBytes, bytes.lengthInBytes);
    } finally {
      image.dispose();
    }
  }))!;
}

void _expectBlue(Uint8List rgba, int Function(int x, int y) expectedBlue) {
  for (var y = 0; y < _height; y++) {
    for (var x = 0; x < _width; x++) {
      final i = (y * _width + x) * 4;
      final expected = expectedBlue(x, y);
      if (rgba[i] != 0 ||
          rgba[i + 1] != 0 ||
          (rgba[i + 2] - expected).abs() > 1 ||
          rgba[i + 3] != 255) {
        fail(
          'pixel ($x,$y) = ${rgba.sublist(i, i + 4)}, expected [0,0,$expected,255]',
        );
      }
    }
  }
}

void main() {
  testWidgets(
    'padded half-opacity stripes keep their width and move with scroll',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final key = GlobalKey();
      await _mount(
        tester,
        key,
        SingleChildScrollView(
          controller: controller,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: List<Widget>.filled(
              10,
              const Opacity(
                opacity: 0.5,
                child: Padding(
                  padding: EdgeInsets.all(5),
                  child: ColoredBox(color: _blue, child: SizedBox(height: 50)),
                ),
              ),
            ),
          ),
        ),
      );
      for (final offset in [0, 17]) {
        if (offset != 0) {
          controller.jumpTo(offset.toDouble());
          await tester.pump();
        }
        _expectBlue(await _read(tester, key), (x, y) {
          final rowY = (y + offset) % 60;
          return x >= 5 && x < 95 && rowY >= 5 && rowY < 55 ? 128 : 0;
        });
      }
    },
  );

  testWidgets('empty fractional Opacity presents unchanged background', (
    tester,
  ) async {
    final key = GlobalKey();
    await _mount(
      tester,
      key,
      const Opacity(opacity: 0.5, child: SizedBox.shrink()),
    );
    _expectBlue(await _read(tester, key), (_, _) => 0);
  });

  testWidgets('group opacity applies once across overlapping opaque children', (
    tester,
  ) async {
    for (final group in [false, true]) {
      final key = GlobalKey();
      final stack = Stack(
        children: [
          for (final left in [8.0, 24.0])
            Positioned(
              left: left,
              top: 10,
              child: Opacity(
                opacity: group ? 1 : 0.5,
                child: const ColoredBox(
                  color: _blue,
                  child: SizedBox(width: 28, height: 20),
                ),
              ),
            ),
        ],
      );
      await _mount(
        tester,
        key,
        group ? Opacity(opacity: 0.5, child: stack) : stack,
      );
      _expectBlue(await _read(tester, key), (x, y) {
        if (y < 10 || y >= 30 || x < 8 || x >= 52) return 0;
        return !group && x >= 24 && x < 36 ? 192 : 128;
      });
    }
  });
}
