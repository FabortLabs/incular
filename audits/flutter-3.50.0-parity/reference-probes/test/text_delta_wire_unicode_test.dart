import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('wire deltas use UTF-16 and apply to the platform oldText', () {
    final delta = TextEditingDelta.fromJSON(<String, Object?>{
      'oldText': 'a😀b',
      'deltaText': 'é',
      'deltaStart': 1,
      'deltaEnd': 3,
      'selectionBase': 3,
      'selectionExtent': 1,
      'selectionAffinity': 'TextAffinity.upstream',
      'selectionIsDirectional': true,
      'composingBase': 1,
      'composingExtent': 2,
    }) as TextEditingDeltaReplacement;
    expect(delta.textReplaced, '😀');
    expect(delta.replacedRange, const TextRange(start: 1, end: 3));
    final value = delta.apply(const TextEditingValue(text: 'superseded'));
    expect(value.text, 'aéb');
    expect(
      value.selection,
      const TextSelection(
        baseOffset: 3,
        extentOffset: 1,
        affinity: TextAffinity.upstream,
        isDirectional: true,
      ),
    );
    expect(value.composing, const TextRange(start: 1, end: 2));
  });

  test('wire deltas default to an invalid selection and no composition', () {
    final delta = TextEditingDelta.fromJSON(<String, Object?>{
      'oldText': 'a😀',
      'deltaText': '',
      'deltaStart': -1,
      'deltaEnd': -1,
    });
    expect(delta, isA<TextEditingDeltaNonTextUpdate>());
    expect(delta.selection.isValid, isFalse);
    expect(delta.selection.affinity, TextAffinity.downstream);
    expect(delta.selection.isDirectional, isFalse);
    expect(delta.composing, TextRange.empty);
    expect(delta.apply(TextEditingValue.empty).text, 'a😀');
  });
}
