import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('word spacing increases width independently of letter spacing', () {
    double width({double word = 0, double letter = 0}) {
      final p = TextPainter(
        text: TextSpan(
          text: 'a a',
          style: TextStyle(
            fontSize: 20,
            wordSpacing: word,
            letterSpacing: letter,
          ),
        ),
        textDirection: TextDirection.ltr,
      )..layout();
      final w = p.width;
      p.dispose();
      return w;
    }

    final base = width(), letter = width(letter: 2), word = width(word: 12);
    print('base=$base letter=$letter word=$word');
    expect(letter, greaterThan(base));
    expect(word - base, closeTo(12, 0.01));
  });
  test('Signed word spacing is applied on each hard-break line', () {
    List<double> widths(double word) {
      final p = TextPainter(
        text: TextSpan(
          text: 'a a\na a',
          style: TextStyle(fontSize: 20, wordSpacing: word),
        ),
        textDirection: TextDirection.ltr,
      )..layout(maxWidth: 240);
      final lines = p.computeLineMetrics().map((line) => line.width).toList();
      p.dispose();
      return lines;
    }

    final base = widths(0);
    expect(base, [60, 60]);
    for (final word in [9.0, -3.0, 0.0, 17.0]) {
      expect(widths(word), [base.first + word, base.last + word]);
    }
  });
}
