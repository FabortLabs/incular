import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('absent and collapsed composing ranges differ at valid boundaries', () {
    const text = 'a😀b';
    expect(const TextEditingValue(text: text).isComposingRangeValid, isFalse);
    for (final range in [
      const TextRange(start: 1, end: 3),
      const TextRange.collapsed(3),
      const TextRange.collapsed(4),
    ]) {
      expect(
        TextEditingValue(text: text, composing: range).isComposingRangeValid,
        isTrue,
      );
    }
  });
}
