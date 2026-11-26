import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

class _IsolatedBinding extends BindingBase
    with SchedulerBinding, ServicesBinding {
  final phases = <String>[];

  @override
  void handleBeginFrame(Duration? rawTimeStamp) {
    phases.add(rawTimeStamp == null ? 'warm begin' : 'native begin');
    super.handleBeginFrame(rawTimeStamp);
  }

  @override
  void handleDrawFrame() {
    phases.add('draw');
    super.handleDrawFrame();
  }
}

class _RecordedTimer implements Timer {
  @override
  bool get isActive => false;
  @override
  int get tick => 0;
  @override
  void cancel() {}
}

void main() {
  test(
    'a native frame colliding with warm-up is requested once after draw',
    () {
      final binding = _IsolatedBinding();
      final platform = binding.platformDispatcher;
      addTearDown(() {
        platform.onBeginFrame = null;
        platform.onDrawFrame = null;
      });

      final warmUp = <VoidCallback>[];
      runZoned(
        binding.scheduleWarmUpFrame,
        zoneSpecification: ZoneSpecification(
          createTimer: (self, parent, zone, duration, callback) {
            warmUp.add(callback);
            return _RecordedTimer();
          },
        ),
      );
      expect(warmUp, hasLength(2));
      warmUp.first();
      binding.ensureFrameCallbacksRegistered();
      expect(binding.phases, ['warm begin']);

      platform.onBeginFrame!(const Duration(milliseconds: 30));
      platform.onDrawFrame!();
      expect(binding.phases, ['warm begin']);
      expect(binding.hasScheduledFrame, isFalse);
      expect(binding.schedulerPhase, SchedulerPhase.midFrameMicrotasks);

      warmUp.last();
      expect(binding.phases, ['warm begin', 'draw']);
      expect(binding.hasScheduledFrame, isTrue);
      expect(binding.schedulerPhase, SchedulerPhase.idle);

      platform.onBeginFrame!(const Duration(milliseconds: 46));
      platform.onDrawFrame!();
      expect(binding.phases, ['warm begin', 'draw', 'native begin', 'draw']);
      expect(binding.hasScheduledFrame, isFalse);
    },
  );
}
