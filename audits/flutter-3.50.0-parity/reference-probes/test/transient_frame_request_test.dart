import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('cancellation does not retract an already requested frame', (
    tester,
  ) async {
    await tester.pump();
    final scheduler = SchedulerBinding.instance;
    expect(scheduler.hasScheduledFrame, isFalse);
    final id = scheduler.scheduleFrameCallback((_) => fail('cancelled'));
    expect(scheduler.hasScheduledFrame, isTrue);
    scheduler.cancelFrameCallbackWithId(id);
    expect(scheduler.hasScheduledFrame, isTrue);
    await tester.pump();
    expect(scheduler.hasScheduledFrame, isFalse);

    final passive = scheduler.scheduleFrameCallback(
      (_) => fail('cancelled passive callback'),
      scheduleNewFrame: false,
    );
    expect(scheduler.hasScheduledFrame, isFalse);
    scheduler.cancelFrameCallbackWithId(passive);
    expect(scheduler.hasScheduledFrame, isFalse);
    await tester.pump();
  });
}
