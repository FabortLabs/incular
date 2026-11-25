import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a single rectangular path contains its closed boundary', () {
    final rect = Rect.fromLTWH(10, 20, 30, 40);
    final regular = Path()..addRect(rect);
    final reverse = Path()
      ..moveTo(40, 60)
      ..lineTo(40, 20)
      ..lineTo(10, 20)
      ..lineTo(10, 60)
      ..close();
    for (final path in [regular, reverse]) {
      for (final rule in PathFillType.values) {
        path.fillType = rule;
        expect(path.getBounds(), rect);
        for (final point in const [
          Offset(10, 20),
          Offset(40, 60),
          Offset(40, 30),
          Offset(10, 35),
          Offset(20, 60),
          Offset(25, 40),
        ]) {
          expect(path.contains(point), isTrue, reason: '$point');
        }
        for (final point in const [
          Offset(9.999, 30),
          Offset(40.001, 30),
          Offset(25, 19.999),
          Offset(25, 60.001),
        ]) {
          expect(path.contains(point), isFalse, reason: '$point');
        }
      }
    }
  });

  test('a compound path retains fill-rule-dependent holes', () {
    final path = Path()
      ..addRect(const Rect.fromLTWH(0, 0, 20, 20))
      ..addRect(const Rect.fromLTWH(5, 5, 10, 10))
      ..fillType = PathFillType.evenOdd;
    expect(path.contains(const Offset(4, 5)), isTrue);
    expect(path.contains(const Offset(10, 10)), isFalse);
    path.fillType = PathFillType.nonZero;
    expect(path.contains(const Offset(10, 10)), isTrue);
  });
}
