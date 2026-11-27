import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'packing extended sRGB clips only after the P3 primaries are converted',
    () {
      const teal = Color.from(
        alpha: 0.5,
        red: 30 / 255,
        green: 202 / 255,
        blue: 211 / 255,
        colorSpace: ColorSpace.displayP3,
      );
      final extended = teal.withValues(colorSpace: ColorSpace.extendedSRGB);
      expect(extended.a, 0.5);
      expect(extended.r, lessThan(-0.37));
      expect(extended.toARGB32(), 0x8000CED5);

      // The direct bounded transform in this pin clamps the input, without
      // applying its child matrix transform. Record it separately.
      expect(
        teal.withValues(colorSpace: ColorSpace.sRGB).toARGB32(),
        0x801ECAD3,
      );
    },
  );
}
