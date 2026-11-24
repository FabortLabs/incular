import 'package:flutter/physics.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('zero bounce gives critical damping', () {
    for (final entry in [
      (-0.1, SpringType.overDamped),
      (0.1, SpringType.underDamped),
      (0.0, SpringType.criticallyDamped),
    ]) {
      final spring = SpringDescription.withDurationAndBounce(
        duration: const Duration(milliseconds: 500),
        bounce: entry.$1,
      );
      final sim = SpringSimulation(spring, 0, 1, 0);
      print('bounce=${entry.$1} type=${sim.type}');
      expect(sim.type, entry.$2);
    }
  });
  test('bounded friction respects tolerance around 8192', () {
    final sim = BoundedFrictionSimulation(.5, 8191, 1, 8190, 8192);
    for (final entry in [(1.700, false), (1.702, true), (1.701, false)]) {
      print(
        'time=${entry.$1} x=${sim.x(entry.$1)} done=${sim.isDone(entry.$1)}',
      );
      expect(sim.isDone(entry.$1), entry.$2);
    }
  });
}
