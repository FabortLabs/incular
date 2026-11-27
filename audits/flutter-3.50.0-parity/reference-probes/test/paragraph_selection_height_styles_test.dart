import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('height styles distinguish natural line boxes, leading, and strut', () {
    for (final (style, strut, lineHeight, spacing, tightY, strutBounds) in [
      (
        const TextStyle(fontFamily: 'FlutterTest', fontSize: 10, height: 2),
        null,
        20.0,
        7.5,
        7.5,
        (0.0, 20.0),
      ),
      (
        const TextStyle(fontFamily: 'FlutterTest', fontSize: 10),
        const StrutStyle(
          fontFamily: 'FlutterTest',
          fontSize: 20,
          height: 1.5,
          leading: 0.2,
          forceStrutHeight: true,
        ),
        34.0,
        7.5,
        17.0,
        (2.0, 32.0),
      ),
      (
        const TextStyle(fontFamily: 'FlutterTest', fontSize: 10),
        const StrutStyle(
          fontFamily: 'FlutterTest',
          fontSize: 20,
          height: 1.5,
          leading: 0.2,
        ),
        34.0,
        9.5,
        17.0,
        (2.0, 32.0),
      ),
    ]) {
      final painter = TextPainter(
        text: TextSpan(text: 'ab\ncd\nef', style: style),
        textDirection: TextDirection.ltr,
        strutStyle: strut,
      )..layout(maxWidth: 40);
      for (final height in ui.BoxHeightStyle.values) {
        final boxes = [
          for (final offset in [0, 3, 6])
            painter
                .getBoxesForSelection(
                  TextSelection(baseOffset: offset, extentOffset: offset + 2),
                  boxHeightStyle: height,
                )
                .single,
        ];
        for (var line = 0; line < boxes.length; ++line) {
          final (top, bottom) = switch (height) {
            ui.BoxHeightStyle.tight => (tightY, tightY + 10),
            ui.BoxHeightStyle.max => (0.0, lineHeight),
            ui.BoxHeightStyle.includeLineSpacingTop => (
              line == 0 ? spacing : 0.0,
              lineHeight,
            ),
            ui.BoxHeightStyle.includeLineSpacingMiddle => (
              line == 0 ? spacing : spacing / 2,
              line == 2 ? lineHeight : lineHeight + spacing / 2,
            ),
            ui.BoxHeightStyle.includeLineSpacingBottom => (
              spacing,
              line == 2 ? lineHeight : lineHeight + spacing,
            ),
            ui.BoxHeightStyle.strut => strutBounds,
          };
          final origin = line * lineHeight;
          expect(
            (boxes[line].top, boxes[line].bottom),
            (origin + top, origin + bottom),
            reason: '$height, strut: $strut, line: $line',
          );
        }
      }
      painter.dispose();
    }
  });
  test('strut boxes without an explicit strut preserve each shaped run', () {
    final painter = TextPainter(
      text: const TextSpan(
        text: 'a',
        style: TextStyle(fontFamily: 'FlutterTest', fontSize: 10, height: 2),
        children: [
          TextSpan(text: 'b', style: TextStyle(fontSize: 20, height: 1.5)),
          TextSpan(text: 'c', style: TextStyle(fontSize: 8, height: 0.5)),
        ],
      ),
      textDirection: TextDirection.ltr,
    )..layout(maxWidth: 80);
    final boxes = painter.getBoxesForSelection(
      const TextSelection(baseOffset: 0, extentOffset: 3),
      boxHeightStyle: ui.BoxHeightStyle.strut,
    );
    expect(boxes.map((box) => (box.top, box.bottom)).toList(), [
      (7.5, 27.5),
      (0.0, 30.0),
      (19.5, 23.5),
    ]);
    painter.dispose();
  });
}
