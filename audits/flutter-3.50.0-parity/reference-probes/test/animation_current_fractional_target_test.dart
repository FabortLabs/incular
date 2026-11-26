import 'package:flutter/animation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool reverse in <bool>[false, true]) {
    testWidgets('current fractional target is immediate (reverse=$reverse)', (
      tester,
    ) async {
      final controller = AnimationController(
        duration: const Duration(seconds: 1),
        value: reverse ? 1 : 0,
        vsync: tester,
      );
      addTearDown(controller.dispose);
      final log = <AnimationStatus>[];
      controller.addStatusListener(log.add);
      final previous = reverse ? controller.reverse() : controller.forward();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 333));
      final double exact = controller.value;
      expect(exact, closeTo(reverse ? 0.667 : 0.333, 1e-12));
      final canceled = expectLater(
        previous.orCancel,
        throwsA(isA<TickerCanceled>()),
      );
      final replacement = controller.animateTo(
        exact,
        duration: const Duration(milliseconds: 100),
      );
      expect(controller.isAnimating, isFalse);
      expect(controller.value, exact);
      expect(log, <AnimationStatus>[
        reverse ? AnimationStatus.reverse : AnimationStatus.forward,
        AnimationStatus.completed,
      ]);
      await replacement;
      await canceled;
    });
  }
}
