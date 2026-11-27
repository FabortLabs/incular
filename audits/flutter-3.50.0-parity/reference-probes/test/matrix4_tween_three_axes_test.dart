import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void expectMatrix(Matrix4 actual, List<double> expected) {
  for (var index = 0; index < 16; index += 1) {
    expect(actual.storage[index], closeTo(expected[index], 1e-12));
  }
}

void main() {
  test('Matrix4Tween blends three axes and reflected scale', () {
    final a = Matrix4.translationValues(2, -5, 7)..scaleByDouble(2, 3, 4, 1);
    final b = Matrix4.translationValues(-9, 11, 13)
      ..rotateY(2.4)
      ..rotateX(-0.4)
      ..scaleByDouble(-3, 0.5, 8, 1);
    final tween = Matrix4Tween(begin: a, end: b);
    expect(tween.transform(0), a);
    expect(tween.transform(1), b);
    final mid = tween.lerp(0.25);
    expect(mid.storage.sublist(12, 15), [-0.75, -1, 8.5]);
    expectMatrix(mid, [
      0.6425838040254901, 0.06868908426300949, -0.3806151133458049, 0,
      -0.2690136225163055, 2.359545123961154, -0.02834570323344445, 0,
      2.515456826394721, 0.33854074452528704, 4.307884297290315, 0,
      -0.75, -1, 8.5, 1,
    ]);
  });

  test('Matrix4Tween normalizes quaternion interpolation without a sign flip', () {
    final a = Matrix4.rotationX(2.9);
    final b = Matrix4.rotationY(-2.9);
    final tween = Matrix4Tween(begin: a, end: b);
    expectMatrix(tween.lerp(0.5), [
      0, 1, 0, 0, 1, 0, 0, 0, 0, 0, -1, 0, 0, 0, 0, 1,
    ]);
  });

  test('Matrix4Tween preserves projective endpoints but interpolates affine components', () {
    final a = Matrix4.identity()..setEntry(3, 2, -0.02);
    final b = Matrix4.translationValues(2, 4, 6)..setEntry(3, 0, 0.01);
    final tween = Matrix4Tween(begin: a, end: b);
    expect(tween.transform(0), a);
    expect(tween.transform(1), b);
    expect(tween.lerp(0.5), Matrix4.translationValues(1, 2, 3));
  });
}
