import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a swatch keeps left key order and fades only missing shades', () {
    const a = ColorSwatch<String>(0xFF106030, {
      'left-only': Color(0xFF207040),
      'shared': Color(0x00000000),
      'another-left': Color(0xFF408060),
    });
    const b = ColorSwatch<String>(0xFF509070, {
      'shared': Color(0xFFFFFFFF),
      'right-only': Color(0xFF60A080),
    });
    final middle = ColorSwatch.lerp(a, b, 0.5)!;
    expect(middle.keys, ['left-only', 'shared', 'another-left']);
    expect(middle['right-only'], isNull);
    expect(middle['left-only']!.toARGB32(), 0x80207040);
    expect(middle['shared']!.toARGB32(), 0x80808080);
    expect(middle.toARGB32(), 0xFF307850);

    final enter = ColorSwatch.lerp(null, b, 0.5)!;
    expect(enter.keys, ['shared', 'right-only']);
    expect(enter['right-only']!.toARGB32(), 0x8060A080);

    final exit = ColorSwatch.lerp(a, null, 0.5)!;
    expect(exit.keys, a.keys);
    expect(exit.toARGB32(), 0x80106030);

    final endpoint = ColorSwatch.lerp(a, b, 1)!;
    expect(endpoint.keys, a.keys);
    expect(endpoint['left-only']!.toARGB32(), 0x00207040);
  });

  test('the packed swatch midpoint rounds channels to 128', () {
    const a = ColorSwatch<int>(0x00000000, {1: Color(0x00000000)});
    const b = ColorSwatch<int>(0xFFFFFFFF, {1: Color(0xFFFFFFFF)});
    final middle = ColorSwatch.lerp(a, b, 0.5)!;
    expect(middle.toARGB32(), 0x80808080);
    expect(middle[1]!.toARGB32(), 0x80808080);

    final nonFinite = ColorSwatch.lerp(a, b, double.nan)!;
    expect(nonFinite.toARGB32(), 0xFFFFFFFF);
    expect(nonFinite[1]!.toARGB32(), 0xFFFFFFFF);
  });
}
