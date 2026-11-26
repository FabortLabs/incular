import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const rect = Rect.fromLTWH(10, 20, 120, 80);

  test('box and stadium hit tests use half-open outer bounds', () {
    for (final ShapeBorder border in <ShapeBorder>[Border.all(), const StadiumBorder()]) {
      expect(border.hitTest(rect, const Offset(10, 60)), isTrue);
      expect(border.hitTest(rect, const Offset(70, 20)), isTrue);
      expect(border.hitTest(rect, const Offset(130, 60)), isFalse);
      expect(border.hitTest(rect, const Offset(70, 100)), isFalse);
      expect(border.hitTest(rect, const Offset(70, 99.999)), isTrue);
      expect(border.hitTest(Rect.zero, Offset.zero), isFalse);
    }
    expect(const StadiumBorder().hitTest(rect, const Offset(11, 21)), isFalse);
  });

  test('circle hit tests use the eccentric fitted ellipse', () {
    const circle = CircleBorder();
    const halfway = CircleBorder(eccentricity: 0.5);
    const oval = OvalBorder();
    for (final (ShapeBorder border, double left, double right) in
        <(ShapeBorder, double, double)>[(circle, 30, 110), (halfway, 20, 120), (oval, 10, 130)]) {
      expect(border.hitTest(rect, Offset(left, 60)), isTrue);
      expect(border.hitTest(rect, Offset(left - 0.001, 60)), isFalse);
      expect(border.hitTest(rect, Offset(right, 60)), isFalse);
      expect(border.hitTest(rect, Offset(right - 0.001, 60)), isTrue);
    }
    expect(oval.hitTest(rect, const Offset(106, 92)), isTrue); // (0.6, 0.8).
    expect(oval.hitTest(rect, const Offset(106.001, 92)), isFalse);
  });

  test('elliptical rounded corners respect shared scaling and logical direction', () {
    const rounded = RoundedRectangleBorder(
      borderRadius: BorderRadiusDirectional.only(
        topStart: Radius.elliptical(60, 40),
        topEnd: Radius.elliptical(90, 8),
      ),
    );
    for (final (TextDirection direction, double start, double inward) in
        <(TextDirection, double, double)>[(TextDirection.ltr, 10, 1), (TextDirection.rtl, 130, -1)]) {
      // 60+90 overflows the width by 25%, so the top-start radii become (48, 32).
      expect(rounded.hitTest(rect, Offset(start + 8 * inward, 38), textDirection: direction), isTrue);
      expect(rounded.hitTest(rect, Offset(start + 8 * inward, 33), textDirection: direction), isFalse);
      expect(rounded.hitTest(rect, const Offset(70, 100), textDirection: direction), isFalse);
    }
  });
}
