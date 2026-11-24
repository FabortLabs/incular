import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'equal-length replacement preserves interior selection and composing',
    () {
      const value = TextEditingValue(
        text: 'éaq',
        selection: TextSelection(baseOffset: 2, extentOffset: 1),
        composing: TextRange(start: 1, end: 2),
      );
      expect(
        value.replaced(const TextRange(start: 0, end: 2), 'b中'),
        const TextEditingValue(
          text: 'b中q',
          selection: TextSelection(baseOffset: 2, extentOffset: 1),
          composing: TextRange(start: 1, end: 2),
        ),
      );
    },
  );

  test('insertion exactly before composing shifts both ends', () {
    const value = TextEditingValue(
      text: 'éa🙂',
      selection: TextSelection.collapsed(offset: 2),
      composing: TextRange(start: 1, end: 4),
    );
    expect(
      value.replaced(const TextRange.collapsed(1), '中'),
      const TextEditingValue(
        text: 'é中a🙂',
        selection: TextSelection.collapsed(offset: 3),
        composing: TextRange(start: 2, end: 5),
      ),
    );
  });

  test('shorter UTF-16 replacement moves interior caret to its end', () {
    const value = TextEditingValue(
      text: 'abcd!',
      selection: TextSelection.collapsed(offset: 2),
      composing: TextRange(start: 1, end: 3),
    );
    expect(
      value.replaced(const TextRange(start: 0, end: 4), '🙂'),
      const TextEditingValue(
        text: '🙂!',
        selection: TextSelection.collapsed(offset: 2),
        composing: TextRange.collapsed(2),
      ),
    );
  });
}
