import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('equal UTF-16 replacement preserves interior offsets', () {
    final toAscii = FilteringTextInputFormatter.deny(
      '🦄a',
      replacementString: 'XYZ',
    );
    final first = toAscii.formatEditUpdate(
      TextEditingValue.empty,
      const TextEditingValue(
        text: 'x🦄ay',
        selection: TextSelection(baseOffset: 4, extentOffset: 3),
        composing: TextRange(start: 1, end: 3),
      ),
    );
    expect(first.text, 'xXYZy');
    expect(
      first.selection,
      const TextSelection(baseOffset: 4, extentOffset: 3),
    );
    expect(first.composing, const TextRange(start: 1, end: 3));

    final toEmoji = FilteringTextInputFormatter.deny(
      'XY',
      replacementString: '🦄',
    );
    final second = toEmoji.formatEditUpdate(
      TextEditingValue.empty,
      const TextEditingValue(
        text: 'aXYz',
        selection: TextSelection(baseOffset: 3, extentOffset: 2),
      ),
    );
    expect(second.text, 'a🦄z');
    expect(
      second.selection,
      const TextSelection(baseOffset: 3, extentOffset: 2),
    );
  });
}
