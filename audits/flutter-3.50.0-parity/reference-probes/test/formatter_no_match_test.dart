import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'digitsOnly removes all nonmatching text and clamps a stranded caret',
    () {
      const oldValue = TextEditingValue(
        text: '42',
        selection: TextSelection.collapsed(offset: 2),
      );
      const proposal = TextEditingValue(
        text: 'X',
        selection: TextSelection.collapsed(offset: 1),
      );
      const expected = TextEditingValue(
        selection: TextSelection.collapsed(offset: 0),
      );
      expect(
        FilteringTextInputFormatter.digitsOnly.formatEditUpdate(
          oldValue,
          proposal,
        ),
        expected,
      );
    },
  );

  test('digitsOnly remaps reverse selection and active composition', () {
    const proposal = TextEditingValue(
      text: '1😀2β3',
      selection: TextSelection(baseOffset: 5, extentOffset: 1),
      composing: TextRange(start: 3, end: 5),
    );
    const expected = TextEditingValue(
      text: '123',
      selection: TextSelection(baseOffset: 2, extentOffset: 1),
      composing: TextRange(start: 1, end: 2),
    );
    expect(
      FilteringTextInputFormatter.digitsOnly.formatEditUpdate(
        TextEditingValue.empty,
        proposal,
      ),
      expected,
    );
  });
}
