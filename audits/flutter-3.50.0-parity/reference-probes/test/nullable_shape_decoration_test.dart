import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('nullable shape decoration scales the fill, corners, border and shadow', () {
    const value = ShapeDecoration(
      color: Color(0xc8325078),
      shape: RoundedRectangleBorder(
        side: BorderSide(width: 8),
        borderRadius: BorderRadius.all(Radius.circular(24)),
      ),
      shadows: <BoxShadow>[
        BoxShadow(
          color: Color(0x80304050),
          offset: Offset(16, 8),
          blurRadius: 8,
          spreadRadius: 4,
        ),
      ],
    );
    final fromNull = ShapeDecoration.lerp(null, value, 0.25)!;
    final toNull = ShapeDecoration.lerp(value, null, 0.75)!;
    expect(fromNull, toNull);
    expect(fromNull.color!.a, closeTo(value.color!.a / 4, 1e-9));
    expect(fromNull.shape.dimensions, const EdgeInsets.all(2));
    expect(
      (fromNull.shape as RoundedRectangleBorder).borderRadius,
      const BorderRadius.all(Radius.circular(6)),
    );
    expect(fromNull.shadows!.single.offset, const Offset(4, 2));
    expect(fromNull.shadows!.single.blurRadius, 2);
    expect(fromNull.shadows!.single.spreadRadius, 1);
    final zero = ShapeDecoration.lerp(value, null, 1)!;
    expect(zero.color!.a, 0);
    expect(zero.shape.dimensions, EdgeInsets.zero);
  });
}
