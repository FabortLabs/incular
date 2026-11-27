import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class ExpectedGrid extends CustomPainter {
  const ExpectedGrid(this.color, this.step, this.xWidths, this.yWidths);

  final Color color;
  final double step;
  final List<double> xWidths;
  final List<double> yWidths;

  @override
  void paint(Canvas canvas, Size size) {
    for (var i = 0; i < xWidths.length; i++) {
      canvas.drawLine(
        Offset(i * step, 0),
        Offset(i * step, size.height),
        Paint()
          ..color = color
          ..strokeWidth = xWidths[i],
      );
    }
    for (var i = 0; i < yWidths.length; i++) {
      canvas.drawLine(
        Offset(0, i * step),
        Offset(size.width, i * step),
        Paint()
          ..color = color
          ..strokeWidth = yWidths[i],
      );
    }
  }

  @override
  bool shouldRepaint(ExpectedGrid oldDelegate) => false;
}

Future<Uint8List> raster(
  WidgetTester tester,
  Size size,
  Widget Function(Widget) paint,
) async {
  final boundary = GlobalKey();
  await tester.pumpWidget(
    Center(
      child: RepaintBoundary(
        key: boundary,
        child: paint(
          SizedBox.fromSize(
            size: size,
            child: const ColoredBox(color: Color(0xFF000000)),
          ),
        ),
      ),
    ),
  );
  return (await tester.runAsync(() async {
    final render =
        boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final image = await render.toImage(pixelRatio: 4);
    try {
      final pixels = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      return Uint8List.fromList(
        pixels.buffer.asUint8List(pixels.offsetInBytes, pixels.lengthInBytes),
      );
    } finally {
      image.dispose();
    }
  }))!;
}

void main() {
  testWidgets(
    'default grid lays down separate ordered strokes over its child',
    (tester) async {
      const size = Size(100, 50);
      final actual = await raster(
        tester,
        size,
        (child) => GridPaper(child: child),
      );
      final expected = await raster(
        tester,
        size,
        (child) => CustomPaint(
          foregroundPainter: const ExpectedGrid(
            Color(0x7FC3E8F3),
            10,
            <double>[1, .25, .5, .25, .5, .25, .5, .25, .5, .25, 1],
            <double>[1, .25, .5, .25, .5, .25],
          ),
          child: child,
        ),
      );
      expect(actual, orderedEquals(expected));
    },
  );

  testWidgets('custom divisions follow the pinned medium line rule', (
    tester,
  ) async {
    const size = Size(12, 6);
    const color = Color(0xBF11FF20);
    final actual = await raster(
      tester,
      size,
      (child) => GridPaper(
        interval: 12,
        divisions: 3,
        subdivisions: 2,
        color: color,
        child: child,
      ),
    );
    final expected = await raster(
      tester,
      size,
      (child) => CustomPaint(
        foregroundPainter: const ExpectedGrid(
          color,
          2,
          <double>[1, .25, .25, .5, .25, .25, 1],
          <double>[1, .25, .25, .5],
        ),
        child: child,
      ),
    );
    expect(actual, orderedEquals(expected));
  });
}
