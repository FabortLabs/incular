import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

TextEditingValue value(String text, int caret) => TextEditingValue(
  text: text,
  selection: TextSelection.collapsed(offset: caret),
);
void main() {
  test('digits filter remaps the caret across removed characters', () {
    final f = FilteringTextInputFormatter.digitsOnly;
    for (final (text, caret) in [('1a2', 2), ('1😀2', 3)]) {
      final got = f.formatEditUpdate(value('12', 1), value(text, caret));
      expect(got.text, '12');
      expect(got.selection.extentOffset, 1);
    }
  });
  test('already full enforced field rejects middle insert without losing its suffix', () {
    final f = LengthLimitingTextInputFormatter(
      3,
      maxLengthEnforcement: MaxLengthEnforcement.enforced,
    );
    final old = value('abc', 1);
    expect(f.formatEditUpdate(old, value('aYc', 2)).text, 'aYc');
    expect(f.formatEditUpdate(old, value('aXbc', 2)), old);
  });
}
