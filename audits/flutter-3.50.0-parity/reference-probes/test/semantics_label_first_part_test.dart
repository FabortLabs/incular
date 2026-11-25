import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('only subsequent opposing parts receive direction marks', () {
    final SemanticsLabelBuilder builder = SemanticsLabelBuilder(
      textDirection: TextDirection.ltr,
    );
    builder.addPart('مرحبا', textDirection: TextDirection.rtl);
    expect(builder.build(), 'مرحبا');
    builder.addPart('שלום', textDirection: TextDirection.rtl);
    expect(builder.build(), 'مرحبا \u202Bשלום\u202C');
  });
}
