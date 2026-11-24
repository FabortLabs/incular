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
}
