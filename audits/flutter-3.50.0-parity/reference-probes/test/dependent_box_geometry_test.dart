import 'dart:ui';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('dependent box flips, breaks fit ties, and clamps the near edge', () {
    for (final (
          Offset target,
          double height,
          bool below,
          double offset,
          Offset expected,
        )
        in [
          (const Offset(0, 85), 20.0, true, 5.0, const Offset(10, 60)),
          (const Offset(100, 15), 20.0, false, 5.0, const Offset(40, 20)),
          (const Offset(50, 50), 90.0, true, 7.0, const Offset(25, 57)),
          (const Offset(50, 50), 90.0, false, 7.0, const Offset(25, 10)),
          (const Offset(50, 120), 120.0, true, 7.0, const Offset(25, 90)),
          (const Offset(50, -10), 100.0, false, 7.0, const Offset(25, 10)),
          (const Offset(double.nan, 50), 20.0, true, 0.0, const Offset(40, 50)),
        ]) {
      expect(
        positionDependentBox(
          size: const Size(100, 100),
          childSize: Size(50, height),
          target: target,
          preferBelow: below,
          verticalOffset: offset,
        ),
        expected,
      );
    }
  });
}
