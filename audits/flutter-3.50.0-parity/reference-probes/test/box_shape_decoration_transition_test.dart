import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('box and shape decorations interpolate rather than switching at halfway', () {
    const a = BoxDecoration(color: Color(0xFF000000), shape: BoxShape.circle);
    const b = ShapeDecoration(
      color: Color(0xFFC86400),
      shape: RoundedRectangleBorder(),
    );
    expect(identical(Decoration.lerp(a, b, 0), a), isTrue);
    expect(identical(Decoration.lerp(a, b, 1), b), isTrue);
    const frame = Rect.fromLTWH(0, 0, 200, 100);
    final atQuarter = Decoration.lerp(a, b, 0.25)! as ShapeDecoration;
    final atHalf = Decoration.lerp(a, b, 0.5)! as ShapeDecoration;
    final atThreeQuarters = Decoration.lerp(a, b, 0.75)! as ShapeDecoration;
    expect(atQuarter.color!.toARGB32(), 0xFF321900);
    expect(atHalf.color!.toARGB32(), 0xFF643200);
    expect(atThreeQuarters.color!.toARGB32(), 0xFF964B00);
    expect(
      atHalf.getClipPath(frame, TextDirection.ltr).getBounds(),
      const Rect.fromLTWH(25, 0, 150, 100),
    );
    final fromShape = Decoration.lerp(b, a, 0.75)! as ShapeDecoration;
    expect(fromShape.color, atQuarter.color);
    expect(
      fromShape.getClipPath(frame, TextDirection.rtl).getBounds(),
      atQuarter.getClipPath(frame, TextDirection.rtl).getBounds(),
    );
  });
}
