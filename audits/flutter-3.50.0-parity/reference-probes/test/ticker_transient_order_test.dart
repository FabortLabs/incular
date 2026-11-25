import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('tickers share scheduling order with other transients', (
    tester,
  ) async {
    final log = <String>[];
    final binding = SchedulerBinding.instance;
    binding.scheduleFrameCallback((_) => log.add('first'));
    final ticker = Ticker((_) => log.add('ticker'));
    addTearDown(ticker.dispose);
    ticker.start();
    binding.scheduleFrameCallback((_) => log.add('last'));
    await tester.pump(const Duration(milliseconds: 16));
    expect(log, ['first', 'ticker', 'last']);

    log.clear();
    binding.scheduleFrameCallback((_) => log.add('later registration'));
    await tester.pump(const Duration(milliseconds: 16));
    expect(log, ['ticker', 'later registration']);
  });

  testWidgets('unmuting during dispatch starts on the next frame at zero', (
    tester,
  ) async {
    final ticks = <Duration>[];
    final ticker = Ticker(ticks.add)..muted = true;
    addTearDown(ticker.dispose);
    ticker.start();
    // Drive an unrelated frame while this ticker has never ticked.
    SchedulerBinding.instance.scheduleFrame();
    await tester.pump(const Duration(milliseconds: 10));
    expect(ticks, isEmpty);
    SchedulerBinding.instance.scheduleFrameCallback(
      (_) => ticker.muted = false,
    );
    await tester.pump(const Duration(milliseconds: 10));
    expect(ticks, isEmpty);
    expect(SchedulerBinding.instance.hasScheduledFrame, isTrue);
    await tester.pump(const Duration(milliseconds: 16));
    expect(ticks, [Duration.zero]);
  });
}
