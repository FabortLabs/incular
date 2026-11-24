import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('BoxShadow extrapolates color alongside geometry', () {
    const a = BoxShadow(
      color: Color(0xff404080),
      offset: Offset(2, 4),
      blurRadius: 4,
      spreadRadius: 4,
    );
    const b = BoxShadow(
      color: Color(0xff808040),
      offset: Offset(6, 8),
      blurRadius: 8,
      spreadRadius: 8,
    );
    final samples = [
      for (final t in [-0.5, 0.5, 1.5]) BoxShadow.lerp(a, b, t)!,
    ];
    expect(samples.map((s) => s.offset), [
      const Offset(0, 2),
      const Offset(4, 6),
      const Offset(8, 10),
    ]);
    expect(samples.map((s) => s.blurRadius), [2, 6, 10]);
    expect(samples.map((s) => s.spreadRadius), [2, 6, 10]);
    expect(samples.map((s) => s.color.toARGB32()), [
      0xff2020a0,
      0xff606060,
      0xffa0a020,
    ]);
  });
}
