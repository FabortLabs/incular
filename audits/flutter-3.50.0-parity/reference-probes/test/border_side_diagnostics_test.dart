// Authored for Incular parity; not an upstream test registration.

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('both border scalar diagnostics use the current precision', () {
    final previous = debugDoublePrecision;
    addTearDown(() => debugDoublePrecision = previous);
    const side = BorderSide(width: 1.25, strokeAlign: 0.125);
    debugDoublePrecision = 2;
    expect(side.toString(), 'BorderSide(width: 1.3, strokeAlign: 0.13)');
    debugDoublePrecision = null;
    expect(side.toString(), 'BorderSide(width: 1.3, strokeAlign: 0.1)');
  });
}
