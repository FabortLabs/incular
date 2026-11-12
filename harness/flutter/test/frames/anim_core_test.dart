import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

import 'recorder.dart';

double _statusCode(AnimationStatus status) => switch (status) {
      AnimationStatus.dismissed => 0,
      AnimationStatus.forward => 1,
      AnimationStatus.reverse => 2,
      AnimationStatus.completed => 3,
    };

class _StepSimulation extends Simulation {
  _StepSimulation(this.start, this.distance, this.duration);

  final double start;
  final double distance;
  final Duration duration;

  @override
  double x(double time) => start + distance * (time / duration.inMicroseconds * 1000000).clamp(0, 1);

  @override
  double dx(double time) => time >= duration.inMicroseconds / 1000000 ? 0 : distance * 1000000 / duration.inMicroseconds;

  @override
  bool isDone(double time) => time >= duration.inMicroseconds / 1000000;
}

void main() {
  frameScenario('anim_core__controller_listener_notifications', (h) async {
    final controller = AnimationController(
      vsync: h.tester,
      duration: const Duration(milliseconds: 160),
      reverseDuration: const Duration(milliseconds: 80),
      lowerBound: 0.2,
      upperBound: 0.8,
      value: 0.25,
    );
    addTearDown(controller.dispose);
    var valueNotifications = 0;
    var statusNotifications = 0;
    var forwardOutcome = 0;
    controller.addListener(() => valueNotifications++);
    controller.addStatusListener((_) => statusNotifications++);
    h.value('value', () => controller.value);
    h.value('status', () => _statusCode(controller.status));
    h.value('is_animating', () => controller.isAnimating ? 1 : 0);
    h.value('value_notifications', () => valueNotifications.toDouble());
    h.value('status_notifications', () => statusNotifications.toDouble());
    h.value('forward_outcome', () => forwardOutcome.toDouble());

    await h.mount((_) => const SizedBox.shrink());
    final forward = controller.forward();
    forward.then<void>((_) => forwardOutcome = 1);
    forward.orCancel.then<void>(
      (_) {},
      onError: (Object _) => forwardOutcome = -1,
    );
    await h.frames(3);
    controller.stop();
    await h.frames(1);

    // Flutter dispatches a value notification for every assignment, even
    // when the assigned value equals the current value.
    controller.value = controller.value;
    await h.frames(1);

    controller.reverse(from: 0.75);
    await h.frames(7);
    controller.reset();
    await h.frames(1);
    controller.animateTo(0.5, duration: const Duration(milliseconds: 80));
    await h.frames(6);
    await h.frames(1);
    controller.value = 1.5;
    await h.frames(1);
  });

  frameScenario('anim_core__animate_to_repeat', (h) async {
    final controller = AnimationController(
      vsync: h.tester,
      duration: const Duration(milliseconds: 160),
      value: 0.75,
    );
    addTearDown(controller.dispose);
    var animateOutcome = 0;
    var repeatOutcome = 0;
    h.value('value', () => controller.value);
    h.value('status', () => _statusCode(controller.status));
    h.value('is_animating', () => controller.isAnimating ? 1 : 0);
    h.value('animate_outcome', () => animateOutcome.toDouble());
    h.value('repeat_outcome', () => repeatOutcome.toDouble());

    await h.mount((_) => const SizedBox.shrink());
    final animate = controller.animateTo(0.25);
    animate.then<void>((_) => animateOutcome = 1);
    animate.orCancel.then<void>((_) {}, onError: (Object _) => animateOutcome = -1);
    await h.frames(7);

    final repeat = controller.repeat(
      min: 0.2,
      max: 0.8,
      reverse: true,
      period: const Duration(milliseconds: 64),
      count: 3,
    );
    repeat.then<void>((_) => repeatOutcome = 1);
    repeat.orCancel.then<void>((_) {}, onError: (Object _) => repeatOutcome = -1);
    await h.frames(14);
  });

  frameScenario('anim_core__animate_with_fling', (h) async {
    final controller = AnimationController(
      vsync: h.tester,
      duration: const Duration(milliseconds: 160),
      value: 0.2,
    );
    addTearDown(controller.dispose);
    var simulationOutcome = 0;
    var flingOutcome = 0;
    h.value('value', () => controller.value);
    h.value('status', () => _statusCode(controller.status));
    h.value('is_animating', () => controller.isAnimating ? 1 : 0);
    h.value('simulation_outcome', () => simulationOutcome.toDouble());
    h.value('fling_outcome', () => flingOutcome.toDouble());

    await h.mount((_) => const SizedBox.shrink());
    final simulation = controller.animateWith(
      _StepSimulation(0.2, 0.4, const Duration(milliseconds: 64)),
    );
    simulation.then<void>((_) => simulationOutcome = 1);
    simulation.orCancel.then<void>((_) {}, onError: (Object _) => simulationOutcome = -1);
    await h.frames(5);

    controller.value = 0.35;
    final fling = controller.fling(velocity: 1.0);
    fling.then<void>((_) => flingOutcome = 1);
    fling.orCancel.then<void>((_) {}, onError: (Object _) => flingOutcome = -1);
    await h.frames(100);
  });

  frameScenario('anim_core__scheduler_callbacks_time_dilation', (h) async {
    final controller = AnimationController(
      vsync: h.tester,
      duration: const Duration(milliseconds: 320),
    );
    var tickerElapsedUs = 0.0;
    final ticker = Ticker((elapsed) {
      tickerElapsedUs = elapsed.inMicroseconds.toDouble();
    });
    addTearDown(controller.dispose);
    addTearDown(ticker.dispose);
    addTearDown(() => timeDilation = 1.0);
    var beginCalls = 0;
    var canceledCalls = 0;
    var postCalls = 0;
    h.value('value', () => controller.value);
    h.value('status', () => _statusCode(controller.status));
    h.value('ticker_elapsed_us', () => tickerElapsedUs);
    h.value('begin_calls', () => beginCalls.toDouble());
    h.value('canceled_calls', () => canceledCalls.toDouble());
    h.value('post_calls', () => postCalls.toDouble());

    await h.mount((_) => const SizedBox.shrink());
    final binding = SchedulerBinding.instance;
    binding.scheduleFrameCallback((_) => beginCalls++);
    final canceled = binding.scheduleFrameCallback((_) => canceledCalls++);
    binding.cancelFrameCallbackWithId(canceled);
    binding.addPostFrameCallback((_) => postCalls++);
    timeDilation = 2.0;
    ticker.start();
    controller.forward();
    await h.frames(8);
    controller.stop();
    ticker.stop();
    await h.frames(1);
    timeDilation = 1.0;
    await h.frames(1);
  });
}
