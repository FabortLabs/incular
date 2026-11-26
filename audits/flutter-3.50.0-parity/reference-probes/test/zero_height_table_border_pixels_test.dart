import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

Future<int> _alphaAtFirstRow(TableBorder border, {List<double> rows = const []}) async {
  const width = 96;
  final recorder = ui.PictureRecorder();
  final canvas = ui.Canvas(recorder);
  border.paint(canvas, const ui.Rect.fromLTWH(8, 4, 80, 0), rows: rows, columns: const []);
  final picture = recorder.endRecording();
  final image = await picture.toImage(width, 10);
  final pixels = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  final alpha = pixels.getUint8((4 * width + 32) * 4 + 3);
  image.dispose();
  picture.dispose();
  return alpha;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('Zero-height tables paint an explicit square edge but not a rounded outer ring', () async {
    expect(await _alphaAtFirstRow(const TableBorder()), 0);
    expect(await _alphaAtFirstRow(TableBorder.all()), 255);
    final rounded = TableBorder.all(borderRadius: const BorderRadius.all(Radius.circular(8)));
    expect(await _alphaAtFirstRow(rounded), 0);
    expect(await _alphaAtFirstRow(rounded, rows: const [0]), greaterThan(0));
  });
}
