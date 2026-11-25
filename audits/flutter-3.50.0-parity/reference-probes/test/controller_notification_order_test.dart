import 'package:flutter/animation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('zero-duration value callbacks observe the previous status', (
    tester,
  ) async {
    final controller = AnimationController(
      vsync: tester,
      duration: const Duration(seconds: 1),
      value: 0.5,
    );
    addTearDown(controller.dispose);
    final trace = <(String, double, AnimationStatus)>[];
    controller.addListener(() {
      trace.add(('value', controller.value, controller.status));
    });
    controller.addStatusListener((status) {
      trace.add(('status', controller.value, status));
    });
    await controller.animateTo(0.75, duration: Duration.zero);
    await controller.animateBack(0.25, duration: Duration.zero);
    expect(trace, const [
      ('value', 0.75, AnimationStatus.forward),
      ('status', 0.75, AnimationStatus.completed),
      ('value', 0.25, AnimationStatus.completed),
      ('status', 0.25, AnimationStatus.dismissed),
    ]);
  });

  testWidgets('removed status callbacks are skipped and new ones wait', (
    tester,
  ) async {
    final controller = AnimationController(
      vsync: tester,
      duration: const Duration(seconds: 1),
    );
    addTearDown(controller.dispose);
    final trace = <(String, AnimationStatus)>[];
    void skipped(AnimationStatus status) => trace.add(('B', status));
    var first = true;
    controller.addStatusListener((status) {
      trace.add(('A', status));
      if (first) {
        first = false;
        controller.removeStatusListener(skipped);
        controller.addStatusListener((status) => trace.add(('C', status)));
      }
    });
    controller.addStatusListener(skipped);
    controller.value = 0.5;
    controller.value = 1;
    expect(trace, const [
      ('A', AnimationStatus.forward),
      ('A', AnimationStatus.completed),
      ('C', AnimationStatus.completed),
    ]);
  });
}
