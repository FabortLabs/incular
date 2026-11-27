// Authored for Incular parity; not an upstream test registration.

import 'dart:ui';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'inline speech ranges use the label length and reset at isolated nodes',
    () {
      const span = TextSpan(
        text: 'a',
        locale: Locale('en'),
        spellOut: true,
        children: [
          TextSpan(text: 'b', semanticsLabel: '🦀'),
          WidgetSpan(child: SizedBox.shrink()),
          TextSpan(text: 'c', semanticsIdentifier: 'separate'),
          TextSpan(text: 'd', spellOut: false),
        ],
      );
      final records = <InlineSpanSemanticsInformation>[];
      span.computeSemanticsInformation(records);
      final combined = combineSemanticsInfo(records);
      expect(combined, hasLength(5));
      expect(combined.first.text, 'ab');
      expect(combined.first.semanticsLabel, 'a🦀');
      expect(combined.first.stringAttributes.map((value) => value.range), [
        const TextRange(start: 0, end: 1),
        const TextRange(start: 0, end: 1),
        const TextRange(start: 1, end: 3),
        const TextRange(start: 1, end: 3),
      ]);
      expect(combined[1].isPlaceholder, isTrue);
      expect(combined[1].stringAttributes, isEmpty);
      expect(combined[2].text, isEmpty);
      expect(combined[3].semanticsIdentifier, 'separate');
      expect(
        combined[3].stringAttributes.first.range,
        const TextRange(start: 0, end: 1),
      );
      final last =
          combined.last.stringAttributes.single as LocaleStringAttribute;
      expect(last.range, const TextRange(start: 0, end: 1));
      expect(last.locale, const Locale('en'));
    },
  );
}
