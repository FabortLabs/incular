import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

List<String> describe(TextEditingDelta delta) {
  final builder = DiagnosticPropertiesBuilder();
  delta.debugFillProperties(builder);
  return builder.properties
      .where((node) => !node.isFiltered(DiagnosticLevel.info))
      .map((node) => node.toString())
      .toList();
}

void main() {
  test('diagnostic coordinates count UTF-16 and keep selection direction', () {
    const replaced = TextEditingDeltaReplacement(
      oldText: 'A🙂B',
      replacementText: 'é',
      replacedRange: TextRange(start: 1, end: 3),
      selection: TextSelection(
        baseOffset: 2,
        extentOffset: 1,
        affinity: TextAffinity.upstream,
        isDirectional: true,
      ),
      composing: TextRange(start: 1, end: 2),
    );
    expect(describe(replaced), [
      'oldText: A🙂B',
      'textReplaced: 🙂',
      'replacementText: é',
      'replacedRange: TextRange(start: 1, end: 3)',
      'selection: TextSelection(baseOffset: 2, extentOffset: 1, isDirectional: true)',
      'composing: TextRange(start: 1, end: 2)',
    ]);
    const selectionCleared = TextEditingDeltaNonTextUpdate(
      oldText: 'A🙂B',
      selection: TextSelection.collapsed(offset: -1),
      composing: TextRange.empty,
    );
    expect(describe(selectionCleared), [
      'oldText: A🙂B',
      'selection: TextSelection.invalid',
      'composing: TextRange(start: -1, end: -1)',
    ]);
  });
}
