import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'zero letter spacing overrides a spacing value but unspecified inherits',
    () {
      const inherited = TextStyle(letterSpacing: 4);
      expect(inherited.merge(const TextStyle()).letterSpacing, 4);
      expect(
        inherited.merge(const TextStyle(letterSpacing: 0)).letterSpacing,
        0,
      );
    },
  );

  test('an empty fallback list clears inherited fallbacks', () {
    const inherited = TextStyle(fontFamilyFallback: <String>['Fallback']);
    expect(inherited.merge(const TextStyle()).fontFamilyFallback, <String>[
      'Fallback',
    ]);
    expect(
      inherited
          .merge(const TextStyle(fontFamilyFallback: <String>[]))
          .fontFamilyFallback,
      isEmpty,
    );
  });
}
