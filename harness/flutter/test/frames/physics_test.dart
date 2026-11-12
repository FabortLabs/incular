import 'package:flutter/physics.dart';
import 'package:flutter/widgets.dart';

import 'recorder.dart';

void main() {
  frameScenario('physics__tolerance__default', (h) async {
    const tolerance = Tolerance.defaultTolerance;
    h.value('distance', () => tolerance.distance);
    h.value('time', () => tolerance.time);
    h.value('velocity', () => tolerance.velocity);
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(3);
  });

  frameScenario('physics__spring_description__damping_regimes', (h) async {
    final under = SpringDescription.withDampingRatio(
      mass: 1,
      stiffness: 100,
      ratio: 0.5,
    );
    final critical = SpringDescription.withDampingRatio(
      mass: 1,
      stiffness: 100,
      ratio: 1,
    );
    final over = SpringDescription.withDampingRatio(
      mass: 1,
      stiffness: 100,
      ratio: 2,
    );
    final description = SpringDescription.withDampingRatio(
      mass: 1,
      stiffness: 100,
      ratio: -0.25,
    );
    const raw = SpringDescription(mass: 0, stiffness: 0, damping: -1);
    final underSimulation = SpringSimulation(under, 0, 1, 0);
    final criticalSimulation = SpringSimulation(critical, 0, 1, 0);
    final overSimulation = SpringSimulation(over, 0, 1, 0);
    h.value('under.damping', () => under.damping);
    h.value('under.type', () => underSimulation.type.index.toDouble());
    h.value('critical.damping', () => critical.damping);
    h.value('critical.type', () => criticalSimulation.type.index.toDouble());
    h.value('over.damping', () => over.damping);
    h.value('over.type', () => overSimulation.type.index.toDouble());
    h.value('negative_ratio.damping', () => description.damping);
    h.value('raw.mass', () => raw.mass);
    h.value('raw.stiffness', () => raw.stiffness);
    h.value('raw.damping', () => raw.damping);
    h.value(
      'negative_ratio.type',
      () => SpringSimulation(description, 0, 1, 0).type.index.toDouble(),
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(2);
  });

  frameScenario('physics__spring_description__duration_bounce', (h) async {
    final spring = SpringDescription.withDurationAndBounce(
      duration: const Duration(milliseconds: 300, microseconds: 500),
      bounce: 0.3,
    );
    h.value('mass', () => spring.mass);
    h.value('stiffness', () => spring.stiffness);
    h.value('damping', () => spring.damping);
    h.value('duration_ms', () => spring.duration.inMilliseconds.toDouble());
    h.value('bounce', () => spring.bounce);
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(2);
  });

  frameScenario('physics__spring__damping_regimes', (h) async {
    final simulations = <String, SpringSimulation>{
      'under': SpringSimulation(
        SpringDescription.withDampingRatio(mass: 1, stiffness: 100, ratio: 0.5),
        0,
        100,
        30,
      ),
      'critical': SpringSimulation(
        SpringDescription.withDampingRatio(mass: 1, stiffness: 100, ratio: 1),
        0,
        100,
        30,
      ),
      'over': SpringSimulation(
        SpringDescription.withDampingRatio(mass: 1, stiffness: 100, ratio: 2),
        0,
        100,
        30,
      ),
    };
    for (final entry in simulations.entries) {
      final name = entry.key;
      final simulation = entry.value;
      h.value('$name.x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
      h.value('$name.dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
      h.value('$name.type', () => simulation.type.index.toDouble());
      h.value(
        '$name.done',
        () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
      );
    }
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(40);
  });

  frameScenario('physics__spring__tolerance_boundary', (h) async {
    final simulation = SpringSimulation(
      SpringDescription.withDampingRatio(mass: 1, stiffness: 100, ratio: 1),
      0.001,
      0,
      0,
    );
    h.value('spring.x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('spring.dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'spring.done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(2);
  });

  frameScenario('physics__friction__ordinary', (h) async {
    final simulation = FrictionSimulation(0.135, 10, 120);
    h.value('friction.x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('friction.dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'friction.done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(40);
  });

  frameScenario('physics__friction__tolerance_boundary', (h) async {
    final simulation = FrictionSimulation(
      0.135,
      0,
      0.001,
      tolerance: const Tolerance(velocity: 0.001),
    );
    h.value('friction.x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('friction.dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'friction.done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(2);
  });

  frameScenario('physics__bounded_friction__position_and_velocity', (h) async {
    final simulation = BoundedFrictionSimulation(0.135, 0, 100, -2, 4);
    h.value('bounded.x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('bounded.dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'bounded.done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(12);
  });

  frameScenario('physics__bounded_friction__distance_tolerance', (h) async {
    final simulation = BoundedFrictionSimulation(0.135, 3.9995, 100, -2, 4);
    h.value('bounded.x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('bounded.dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'bounded.done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(2);
  });

  frameScenario('physics__gravity__negative_acceleration_threshold', (h) async {
    final simulation = GravitySimulation(-10, 0, 100, 0);
    h.value('gravity.x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('gravity.dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'gravity.done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(30);
  });

  frameScenario('physics__clamped_simulation__position_and_velocity', (
    h,
  ) async {
    final simulation = ClampedSimulation(
      GravitySimulation(100, 0, 50, 200),
      xMin: 0,
      xMax: 20,
      dxMin: -50,
      dxMax: 50,
    );
    h.value('clamped.x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('clamped.dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'clamped.done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(30);
  });

  frameScenario('physics__bouncing_scroll__extent_handoff', (h) async {
    final simulation = BouncingScrollSimulation(
      position: 90,
      velocity: 3000,
      leadingExtent: 0,
      trailingExtent: 100,
      spring: SpringDescription.withDampingRatio(
        mass: 0.5,
        stiffness: 100,
        ratio: 1.1,
      ),
      tolerance: const Tolerance(distance: 1, velocity: 20),
    );
    h.value('x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.pump(const Duration(milliseconds: 1));
    await h.frames(120);
  });

  frameScenario('physics__scroll_spring__snap_when_done', (h) async {
    final simulation = ScrollSpringSimulation(
      SpringDescription.withDampingRatio(mass: 1, stiffness: 100, ratio: 1.5),
      0,
      100,
      0,
    );
    h.value(
      'scroll_spring.x',
      () => simulation.x(h.elapsed.inMicroseconds / 1e6),
    );
    h.value(
      'scroll_spring.dx',
      () => simulation.dx(h.elapsed.inMicroseconds / 1e6),
    );
    h.value(
      'scroll_spring.done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(100);
  });

  frameScenario('physics__clamping_scroll__spline', (h) async {
    final simulation = ClampingScrollSimulation(position: 70, velocity: 500);
    h.value('x', () => simulation.x(h.elapsed.inMicroseconds / 1e6));
    h.value('dx', () => simulation.dx(h.elapsed.inMicroseconds / 1e6));
    h.value(
      'done',
      () => simulation.isDone(h.elapsed.inMicroseconds / 1e6) ? 1 : 0,
    );
    await h.mount((_) => const SizedBox.shrink());
    await h.frames(20);
  });
}
