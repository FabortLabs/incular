import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  final binding = SchedulerBinding.instance;
  void frame(int ms) {
    binding.handleBeginFrame(Duration(milliseconds: ms));
    binding.handleDrawFrame();
  }

  test('Callback registration respects active phase snapshots', () {
    final events = <String>[];
    var first = true;
    binding.scheduleFrameCallback((_) {
      events.add('transient');
      binding.scheduleFrameCallback((_) => events.add('next-transient'));
    });
    binding.addPersistentFrameCallback((_) {
      events.add('persistent');
      if (first) {
        first = false;
        binding.addPostFrameCallback((_) => events.add('post-from-persistent'));
      }
    });
    binding.addPostFrameCallback((_) {
      events.add('post');
      binding.addPostFrameCallback((_) => events.add('next-post'));
    });

    frame(0);
    final firstFrame = events.toList();
    events.clear();
    frame(16);

    expect(
      <List<String>>[firstFrame, events],
      <List<String>>[
        <String>['transient', 'persistent', 'post', 'post-from-persistent'],
        <String>['next-transient', 'persistent', 'next-post'],
      ],
    );
  });

  test('Current timestamp is visible in each callback phase', () {
    final visible = <(String, bool)>[];
    void record(String phase, Duration callbackTime) =>
        visible.add((phase, binding.currentFrameTimeStamp == callbackTime));

    binding.scheduleFrameCallback((time) => record('transient', time));
    binding.addPersistentFrameCallback((time) => record('persistent', time));
    binding.addPostFrameCallback((time) => record('post', time));
    frame(16);

    expect(visible, <(String, bool)>[
      ('transient', true),
      ('persistent', true),
      ('post', true),
    ]);
    expect(binding.schedulerPhase, SchedulerPhase.idle);
  });
}
