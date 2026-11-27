// Authored for Incular parity; not an upstream test registration.

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('inline widget offsets share UTF-16 coordinates with text affinity', () {
    const root = TextSpan(
      text: 'a🦀',
      children: [
        WidgetSpan(child: SizedBox.shrink()),
        TextSpan(text: 'c'),
      ],
    );
    const down = TextAffinity.downstream;
    const up = TextAffinity.upstream;
    for (final (offset, affinity, expected) in [
      (0, down, 'a🦀'),
      (0, up, null),
      (2, down, 'a🦀'),
      (2, up, 'a🦀'),
      (3, down, '\uFFFC'),
      (3, up, 'a🦀'),
      (4, down, 'c'),
      (4, up, null),
      (5, down, null),
      (5, up, 'c'),
    ]) {
      final span = root.getSpanForPosition(
        TextPosition(offset: offset, affinity: affinity),
      );
      expect(switch (span) {
        TextSpan() => span.text,
        WidgetSpan() => '\uFFFC',
        _ => null,
      }, expected);
    }
  });
}
