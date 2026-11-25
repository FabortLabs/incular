import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('ordinal keys treat signed zeros as ties and names as UTF-16', () {
    expect(const OrdinalSortKey(0).compareTo(const OrdinalSortKey(-0.0)), 0);
    expect(const OrdinalSortKey(-0.0).compareTo(const OrdinalSortKey(0)), 0);
    expect(
      const OrdinalSortKey(
        9,
        name: '😀',
      ).compareTo(const OrdinalSortKey(-9, name: '\uE000')),
      lessThan(0),
    );
  });
}
