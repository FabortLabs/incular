import 'package:flutter/physics.dart';
import 'package:flutter_test/flutter_test.dart';

class Constant extends Simulation {
  Constant(this.p, this.v);
  final double p, v;
  @override
  double x(double t) => p;
  @override
  double dx(double t) => v;
  @override
  bool isDone(double t) => false;
}

void main() {
  test('clamp NaN maps to upper bound without changing completion', () {
    ClampedSimulation bounded(double x, double v) =>
        ClampedSimulation(Constant(x, v), xMin: 0, xMax: 5, dxMin: 0, dxMax: 7);
    final control = bounded(-1, 100);
    expect([control.x(0), control.dx(0)], [0, 7]);
    final nan = bounded(double.nan, double.nan);
    expect([nan.x(0), nan.dx(0)], [5, 7]);
    expect(nan.isDone(0), isFalse);
  });
  test('inverted clamp bounds are rejected in debug', () {
    expect(
      () => ClampedSimulation(Constant(0, 0), xMin: 5, xMax: 0),
      throwsA(isAssertionError),
    );
    expect(
      () => ClampedSimulation(Constant(0, 0), dxMin: 5, dxMax: 0),
      throwsA(isAssertionError),
    );
  });
}
