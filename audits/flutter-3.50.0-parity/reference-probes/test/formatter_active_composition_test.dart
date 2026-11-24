import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

TextEditingValue edit(
  String text,
  int caret, [
  TextRange composing = TextRange.empty,
]) => TextEditingValue(
  text: text,
  selection: TextSelection.collapsed(offset: caret),
  composing: composing,
);

void main() {
  test('length enforcement defers composition only when previously under the limit', () {
    final enforced = LengthLimitingTextInputFormatter(
      2,
      maxLengthEnforcement: MaxLengthEnforcement.enforced,
    );
    final deferred = LengthLimitingTextInputFormatter(
      2,
      maxLengthEnforcement: MaxLengthEnforcement.truncateAfterCompositionEnds,
    );
    final unlimited = LengthLimitingTextInputFormatter(
      2,
      maxLengthEnforcement: MaxLengthEnforcement.none,
    );
    final incoming = edit('abc', 3, const TextRange(start: 1, end: 3));
    final initial = edit('a', 1);
    expect(
      enforced.formatEditUpdate(initial, incoming),
      edit('ab', 2, const TextRange(start: 1, end: 2)),
    );
    expect(deferred.formatEditUpdate(initial, incoming), incoming);
    expect(deferred.formatEditUpdate(incoming, edit('abc', 3)), edit('ab', 2));
    expect(unlimited.formatEditUpdate(initial, incoming), incoming);

    final atLimit = edit('ab', 1);
    final insert = edit('aXb', 2, const TextRange(start: 1, end: 2));
    expect(enforced.formatEditUpdate(atLimit, insert), atLimit);
    expect(deferred.formatEditUpdate(atLimit, insert), atLimit);
    expect(unlimited.formatEditUpdate(atLimit, insert), insert);
  });

  test('oversize replacement uses different enforced and deferred guards', () {
    final enforced = LengthLimitingTextInputFormatter(
      2,
      maxLengthEnforcement: MaxLengthEnforcement.enforced,
    );
    final deferred = LengthLimitingTextInputFormatter(
      2,
      maxLengthEnforcement: MaxLengthEnforcement.truncateAfterCompositionEnds,
    );
    const selected = TextEditingValue(
      text: 'ab',
      selection: TextSelection(baseOffset: 0, extentOffset: 1),
    );
    expect(enforced.formatEditUpdate(selected, edit('aXYZ', 4)), edit('aX', 2));
    expect(deferred.formatEditUpdate(selected, edit('aXYZ', 4)), selected);

    final atLimit = edit('🦄b', 2);
    expect(enforced.formatEditUpdate(atLimit, edit('🦄Xb', 3)), atLimit);
    expect(deferred.formatEditUpdate(atLimit, edit('🦄Xb', 3)), atLimit);
    expect(enforced.formatEditUpdate(atLimit, edit('🦄X', 3)), edit('🦄X', 3));
    expect(deferred.formatEditUpdate(atLimit, edit('🦄X', 3)), edit('🦄X', 3));
  });
}
