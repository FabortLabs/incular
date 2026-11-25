import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a transient callback cancels a later member of the same batch', (
    tester,
  ) async {
    final scheduler = SchedulerBinding.instance;
    final trace = <String>[];
    late final int cancelled;
    scheduler.scheduleFrameCallback((_) {
      trace.add('A');
      scheduler.cancelFrameCallbackWithId(cancelled);
      scheduler.scheduleFrameCallback((_) => trace.add('D'));
    });
    cancelled = scheduler.scheduleFrameCallback((_) => trace.add('B'));
    scheduler.scheduleFrameCallback((_) => trace.add('C'));
    await tester.pump();
    expect(trace, ['A', 'C']);
    await tester.pump(const Duration(milliseconds: 16));
    expect(trace, ['A', 'C', 'D']);
  });

  testWidgets(
    'a ticker started in a post-frame callback starts its clock next frame',
    (tester) async {
      final scheduler = SchedulerBinding.instance;
      final elapsed = <Duration>[];
      final ticker = Ticker(elapsed.add);
      addTearDown(ticker.dispose);
      scheduler.addPostFrameCallback((_) => ticker.start());
      scheduler.scheduleFrame();
      await tester.pump();
      expect(elapsed, isEmpty);
      await tester.pump(const Duration(milliseconds: 16));
      await tester.pump(const Duration(milliseconds: 16));
      expect(elapsed, [Duration.zero, const Duration(milliseconds: 16)]);
    },
  );
}
