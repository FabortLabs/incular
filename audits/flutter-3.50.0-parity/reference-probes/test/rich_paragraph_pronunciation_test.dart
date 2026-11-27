// Authored for Incular parity; not an upstream test registration.

import 'dart:ui';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'mounted rich text publishes UTF-16 pronunciation on the spoken label',
    (WidgetTester tester) async {
      const span = TextSpan(
        text: 'a',
        spellOut: true,
        children: [
          TextSpan(text: 'b', semanticsLabel: '🦀', locale: Locale('es', 'MX')),
        ],
      );
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: RichText(text: span),
        ),
      );
      final label = tester
          .getSemantics(find.byType(RichText))
          .getSemanticsData()
          .attributedLabel;
      expect(label.string, 'a🦀');
      final attrs = label.attributes.toList();
      expect(attrs, hasLength(3));
      expect(attrs[0], isA<SpellOutStringAttribute>());
      expect(attrs[1], isA<SpellOutStringAttribute>());
      expect(attrs[0].range, const TextRange(start: 0, end: 1));
      expect(attrs[1].range, const TextRange(start: 1, end: 3));
      expect(attrs[2].range, const TextRange(start: 1, end: 3));
      expect(
        (attrs[2] as LocaleStringAttribute).locale,
        const Locale('es', 'MX'),
      );
    },
  );
}
