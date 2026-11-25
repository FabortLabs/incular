import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('An intermediate eccentricity fits half the long-axis margin', () {
    const side = BorderSide(width: 10);
    const start = CircleBorder(side: side);
    const finish = CircleBorder(side: BorderSide(width: 20), eccentricity: 1);
    final medium = start.copyWith(eccentricity: 0.5);
    final wide = medium.getOuterPath(const Rect.fromLTWH(0, 0, 200, 100));
    final tall = medium.getOuterPath(const Rect.fromLTWH(10, 20, 100, 200));
    expect(wide.getBounds(), const Rect.fromLTWH(25, 0, 150, 100));
    expect(tall.getBounds(), const Rect.fromLTWH(10, 45, 100, 150));
    expect(finish.scale(0.5), start.copyWith(eccentricity: 1));
    expect(
      (ShapeBorder.lerp(start, finish, 0.5)! as CircleBorder).eccentricity,
      0.5,
    );
    expect(
      (ShapeBorder.lerp(start, finish, -1)! as CircleBorder).eccentricity,
      0,
    );
    expect(
      (ShapeBorder.lerp(start, finish, 2)! as CircleBorder).eccentricity,
      1,
    );
  });
}
