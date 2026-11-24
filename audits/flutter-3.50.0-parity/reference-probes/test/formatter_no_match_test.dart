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
}
