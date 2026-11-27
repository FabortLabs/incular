import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('WidgetSpan only line has no extra font height', () {
    final painter = TextPainter(
      textDirection: TextDirection.ltr,
      text: const TextSpan(
        children: [
          TextSpan(text: 'Start\n', style: TextStyle(height: 1, fontSize: 16)),
          WidgetSpan(child: SizedBox()),
          TextSpan(text: 'End', style: TextStyle(height: 1, fontSize: 16)),
        ],
      ),
    );
    addTearDown(painter.dispose);
    painter.setPlaceholderDimensions(const [
      PlaceholderDimensions(
        size: Size(800, 16),
        alignment: ui.PlaceholderAlignment.bottom,
      ),
    ]);
    painter.layout(maxWidth: 800);
    final lines = painter.computeLineMetrics();
    final box = painter.inlinePlaceholderBoxes!.single;
    expect(lines.map((line) => line.height), [16, 16, 16]);
    expect(box.top, 16);
    expect(box.bottom, 32);

    for (final height in [4.0, 10.0, 14.0, 18.0, 32.0]) {
      painter.setPlaceholderDimensions([
        PlaceholderDimensions(
          size: Size(800, height),
          alignment: ui.PlaceholderAlignment.bottom,
        ),
      ]);
      painter.layout(maxWidth: 800);
      expect(painter.computeLineMetrics()[1].height, height);
    }
  });
}
