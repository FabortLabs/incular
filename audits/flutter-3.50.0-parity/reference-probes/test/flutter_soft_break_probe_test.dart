import 'dart:ui' show TextDirection;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('discretionary hyphen changes paint width but not line caret metrics', () {
    final painter = TextPainter(
      text: const TextSpan(text: 'abc\u00ADde', style: TextStyle(fontSize: 10)),
      textDirection: TextDirection.ltr,
      textWidthBasis: TextWidthBasis.longestLine,
    );
    addTearDown(painter.dispose);

    for (final hyphens in Hyphens.values) {
      painter.hyphens = hyphens;
      for (final (width, expected) in <(double, double)>[
        (30, 30),
        (35, hyphens == Hyphens.manual ? 35 : 30),
        (49, hyphens == Hyphens.manual ? 40 : 30),
        (50, 50),
      ]) {
        painter.layout(maxWidth: width);
        expect(painter.width, expected);
        expect(
          painter.computeLineMetrics().map((line) => line.width),
          width < 50 ? <double>[30, 20] : <double>[50],
        );
        final upstream = painter.getOffsetForCaret(
          const TextPosition(offset: 4, affinity: TextAffinity.upstream),
          Rect.zero,
        );
        final downstream = painter.getOffsetForCaret(const TextPosition(offset: 4), Rect.zero);
        expect(upstream.dx, 30);
        expect(downstream.dx, width < 50 ? 0 : 30);
      }
    }

    painter.hyphens = Hyphens.manual;
    for (final (text, width, widths) in <(String, double, List<double>)>[
      ('abc\u00AD', 30, <double>[30]),
      ('abc\u00AD\nde', 40, <double>[30, 20]),
    ]) {
      painter.text = TextSpan(text: text, style: const TextStyle(fontSize: 10));
      painter.layout(maxWidth: 49);
      expect(painter.width, width);
      expect(painter.computeLineMetrics().map((line) => line.width), widths);
    }
  });

  test('breakable Unicode separators hang on wrapped and hard-ended lines', () {
    for (final code in <int>[0x1680, 0x2000, 0x2009]) {
      final space = String.fromCharCode(code);
      for (final (text, widths, hardBreaks) in <(String, List<double>, List<bool>)>[
        ('A$space', <double>[14], <bool>[true]),
        ('A$space\nB', <double>[14, 14], <bool>[true, true]),
        ('A${space}B', <double>[14, 14], <bool>[false, true]),
      ]) {
        final painter = TextPainter(text: TextSpan(text: text), textDirection: TextDirection.ltr)
          ..layout(maxWidth: 14);
        final metrics = painter.computeLineMetrics();
        expect(metrics.map((line) => line.width), widths);
        expect(metrics.map((line) => line.hardBreak), hardBreaks);
        painter.dispose();
      }
    }
  });
}
