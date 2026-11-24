import 'package:flutter/animation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('removed controller listener does not run in same notification', (
    t,
  ) async {
    final c = AnimationController(
      vsync: t,
      duration: const Duration(seconds: 1),
    );
    try {
      final calls = <String>[];
      void b() => calls.add('B');
      c.addListener(() {
        calls.add('A');
        c.removeListener(b);
      });
      c.addListener(b);
      c.value = 0.5;
      expect(calls, ['A']);
    } finally {
      c.dispose();
    }
  });
  testWidgets(
    'same-target animation notifies terminal status, not an unchanged value',
    (t) async {
      final c = AnimationController(
        vsync: t,
        duration: const Duration(seconds: 1),
        value: .5,
      );
      try {
        final statuses = <AnimationStatus>[];
        var values = 0;
        c.addListener(() => values++);
        c.addStatusListener(statuses.add);
        c.animateTo(.5);
        expect(c.value, .5);
        expect(statuses, [AnimationStatus.completed]);
        expect(values, 0);
      } finally {
        c.dispose();
      }
    },
  );
  testWidgets('previous per-run curve does not affect repeat', (t) async {
    final c = AnimationController(
      vsync: t,
      duration: const Duration(seconds: 1),
    );
    try {
      c.animateTo(1, curve: Curves.decelerate);
      c.stop();
      c.value = 0;
      c.repeat();
      await t.pump();
      await t.pump(const Duration(milliseconds: 250));
      expect(c.value, closeTo(.25, .0001));
    } finally {
      c.dispose();
    }
  });
}
