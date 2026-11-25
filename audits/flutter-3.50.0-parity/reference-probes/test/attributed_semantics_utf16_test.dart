import 'dart:ui';

import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('right-side speech attributes shift by UTF-16 units', () {
    final AttributedString before = AttributedString(
      '😀x',
      attributes: [
        SpellOutStringAttribute(range: const TextRange(start: 0, end: 2)),
      ],
    );
    final AttributedString after = AttributedString(
      'é',
      attributes: [
        LocaleStringAttribute(
          range: const TextRange(start: 0, end: 1),
          locale: const Locale('fr'),
        ),
      ],
    );

    final AttributedString result = before + after;
    expect(result.string, '😀xé');
    expect(result.attributes[0].range, const TextRange(start: 0, end: 2));
    expect(result.attributes[1].range, const TextRange(start: 3, end: 4));
    expect(
      (result.attributes[1] as LocaleStringAttribute).locale,
      const Locale('fr'),
    );
    expect(after.attributes[0].range, const TextRange(start: 0, end: 1));
  });
}
