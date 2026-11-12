import 'package:flutter/widgets.dart';

import 'recorder.dart';

void main() {
  frameScenario('anim_compose__curved_animation__unobserved_reversal', (
    h,
  ) async {
    final controller = AnimationController(
      vsync: h.tester,
      duration: const Duration(milliseconds: 100),
    );
    final curved = CurvedAnimation(
      parent: controller,
      curve: Curves.easeIn,
      reverseCurve: Curves.easeOut,
    );
    addTearDown(() {
      curved.dispose();
      controller.dispose();
    });
    h.value('curved', () => curved.value);
    await h.mount((_) => const SizedBox.shrink());
    controller.forward();
    controller.value = 0.5;
    controller.reverse();
    await h.frames(1);
    controller.stop();
  });
  frameScenario('anim_compose__train_hopping__unobserved_crossing', (h) async {
    final current = AnimationController(vsync: h.tester, value: 0.2);
    final next = AnimationController(vsync: h.tester, value: 0.8);
    var switches = 0;
    final train = TrainHoppingAnimation(
      current,
      next,
      onSwitchedTrain: () {
        switches++;
      },
    );
    addTearDown(() {
      train.dispose();
      current.dispose();
      next.dispose();
    });
    h.value('switches', () => switches.toDouble());
    await h.mount((_) => const SizedBox.shrink());
    next.value = 0.1;
    await h.frames(1);
    train.currentTrain;
    await h.frames(1);
  });
}
