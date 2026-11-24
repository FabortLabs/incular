import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';

void main() {
  test('axis-aligned rectangle bounds and translation controls', () {
    final rectangle = Path()..addRect(const Rect.fromLTWH(10, 10, 40, 30));
    expect(rectangle.getBounds(), const Rect.fromLTWH(10, 10, 40, 30));
    expect(
      rectangle.shift(const Offset(2, 5)).getBounds(),
      const Rect.fromLTWH(12, 15, 40, 30),
    );
  });

  test('Path.getBounds includes quadratic and cubic control points', () {
    final quadratic = Path()
      ..moveTo(10, 10)
      ..quadraticBezierTo(30, 50, 50, 10)
      ..close();
    final cubic = Path()
      ..moveTo(10, 10)
      ..cubicTo(20, 70, 40, 70, 50, 10)
      ..close();
    for (final path in [quadratic, cubic]) {
      expect(path.contains(const Offset(30, 20)), isTrue);
      expect(path.contains(const Offset(30, 60)), isFalse);
    }
    expect(quadratic.getBounds(), const Rect.fromLTRB(10, 10, 50, 50));
    expect(cubic.getBounds(), const Rect.fromLTRB(10, 10, 50, 70));
  });
}
