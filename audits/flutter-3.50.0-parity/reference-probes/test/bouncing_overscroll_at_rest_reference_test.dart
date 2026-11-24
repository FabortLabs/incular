import 'package:flutter/physics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const physics = BouncingScrollPhysics();
  const tolerance = 0.001;
  FixedScrollMetrics metrics(double pixels) => FixedScrollMetrics(
    minScrollExtent: 0,
    maxScrollExtent: 100,
    pixels: pixels,
    viewportDimension: 50,
    axisDirection: AxisDirection.down,
    devicePixelRatio: 1,
  );

  Simulation overscroll() {
    final position = metrics(110);
    expect(physics.toleranceFor(position).distance, 1);
    expect(physics.toleranceFor(position).velocity, 20);
    expect(physics.createBallisticSimulation(metrics(90), 0), isNull);
    return physics.createBallisticSimulation(position, 0)!;
  }

  test('stationary overscroll follows the rubber-band trajectory', () {
    final motion = overscroll();
    expect(motion.x(0), closeTo(110, tolerance));
    expect(motion.dx(0), closeTo(0, tolerance));
    expect(motion.x(0.016), closeTo(108.5348813261472, tolerance));
    expect(motion.dx(0.016), closeTo(-84.51327039469352, tolerance));
  });

  test('stationary overscroll finishes and snaps by 240 ms', () {
    final motion = overscroll();
    expect(motion.isDone(0.224), isFalse);
    expect(motion.isDone(0.320), isTrue);
    expect(motion.isDone(0.240), isTrue);
    expect(motion.x(0.240), 100);
    expect(motion.dx(0.240), closeTo(-9.19663439352646, tolerance));
  });
}
