import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'fixed and significant diagnostics handle decimal ties and exponents',
    () {
      addTearDown(() => debugDoublePrecision = null);
      expect(debugFormatDouble(1.25), '1.3');
      expect(debugFormatDouble(-1.25), '-1.3');
      expect(debugFormatDouble(-0.0), '-0.0');
      expect(debugFormatDouble(1e21), '1e+21');
      expect(debugFormatDouble(double.infinity), 'Infinity');
      expect(debugFormatDouble(double.nan), 'NaN');

      debugDoublePrecision = 3;
      expect(debugFormatDouble(1.125), '1.13');
      expect(debugFormatDouble(-1.125), '-1.13');
      expect(debugFormatDouble(1e-6), '0.00000100');
      expect(debugFormatDouble(1e-7), '1.00e-7');
      expect(debugFormatDouble(1e3), '1.00e+3');
      expect(debugFormatDouble(-0.0), '-0.00');
      debugDoublePrecision = 21;
      final value = 1 + 1 / (1 << 21);
      expect(debugFormatDouble(value), '1.00000047683715820313');
      expect(value.toStringAsFixed(20), '1.00000047683715820313');
    },
  );
}
