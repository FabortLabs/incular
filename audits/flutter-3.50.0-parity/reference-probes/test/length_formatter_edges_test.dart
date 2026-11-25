import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('truncation orders the endpoints of a reversed selection', () {
    const input = TextEditingValue(
      text: 'éabc',
      selection: TextSelection(baseOffset: 4, extentOffset: 1),
      composing: TextRange(start: 2, end: 4),
    );
    final output = LengthLimitingTextInputFormatter.truncate(input, 2);
    expect(output.text, 'éa');
    expect(
      output.selection,
      const TextSelection(baseOffset: 1, extentOffset: 2),
    );
    expect(output.composing, TextRange.empty);
  });

  test('a valid collapsed composing range defers truncation until commit', () {
    final limiter = LengthLimitingTextInputFormatter(
      2,
      maxLengthEnforcement: MaxLengthEnforcement.truncateAfterCompositionEnds,
    );
    const collapsed = TextRange.collapsed(1);
    const composing = TextEditingValue(
      text: 'abc',
      selection: TextSelection.collapsed(offset: 3),
      composing: collapsed,
    );
    expect(
      limiter.formatEditUpdate(const TextEditingValue(text: 'a'), composing),
      composing,
    );
    expect(
      limiter.formatEditUpdate(
        const TextEditingValue(text: 'ab', composing: collapsed),
        const TextEditingValue(
          text: 'aXY',
          selection: TextSelection.collapsed(offset: 3),
        ),
      ),
      const TextEditingValue(
        text: 'aX',
        selection: TextSelection.collapsed(offset: 2),
      ),
    );
  });
}
