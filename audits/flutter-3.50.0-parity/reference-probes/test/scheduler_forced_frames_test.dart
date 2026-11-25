import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

Future<void> _lifecycle(AppLifecycleState state) async {
  final data = const StringCodec().encodeMessage(state.toString());
  await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .handlePlatformMessage('flutter/lifecycle', data, (_) {});
}

void main() {
  testWidgets('a forced frame delivers paused ordinary callbacks once', (
    tester,
  ) async {
    final binding = SchedulerBinding.instance;
    await tester.pump();
    await _lifecycle(AppLifecycleState.paused);
    addTearDown(() => _lifecycle(AppLifecycleState.resumed));

    int calls = 0;
    binding.scheduleFrameCallback((_) => calls++);
    expect(binding.framesEnabled, isFalse);
    expect(binding.hasScheduledFrame, isFalse);
    binding.scheduleForcedFrame();
    binding.scheduleForcedFrame();
    expect(binding.hasScheduledFrame, isTrue);
    await tester.pump(const Duration(milliseconds: 10));
    expect(calls, 1);
    expect(binding.hasScheduledFrame, isFalse);
  });

  testWidgets('forced ticker requests keep their clock while paused', (
    tester,
  ) async {
    final binding = SchedulerBinding.instance;
    await tester.pump();
    await _lifecycle(AppLifecycleState.paused);
    addTearDown(() => _lifecycle(AppLifecycleState.resumed));

    final ticks = <Duration>[];
    final ticker = Ticker(ticks.add)..forceFrames = true;
    addTearDown(ticker.dispose);
    final completed = ticker.start();
    expect(ticker.isActive, isTrue);
    expect(ticker.isTicking, isFalse);
    expect(binding.hasScheduledFrame, isTrue);
    await tester.pump(const Duration(milliseconds: 10));
    expect(binding.hasScheduledFrame, isTrue);
    await tester.pump(const Duration(milliseconds: 10));
    expect(ticks, [Duration.zero, const Duration(milliseconds: 10)]);

    ticker.stop();
    await completed;
    await tester.pump(const Duration(milliseconds: 10));
    expect(ticks, hasLength(2));
    expect(binding.hasScheduledFrame, isFalse);
  });
}
