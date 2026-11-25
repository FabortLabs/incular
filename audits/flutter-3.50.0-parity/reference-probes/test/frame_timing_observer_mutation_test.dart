import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Timing reports snapshot additions and skip removed callbacks', (
    WidgetTester tester,
  ) async {
    final scheduler = SchedulerBinding.instance;
    final seen = <String>[];
    void added(List<FrameTiming> timing) => seen.add('new');
    void removed(List<FrameTiming> timing) => seen.add('removed');
    void last(List<FrameTiming> timing) => seen.add('last');
    var firstBatch = true;
    void first(List<FrameTiming> timing) {
      seen.add('first');
      if (firstBatch) {
        firstBatch = false;
        scheduler.removeTimingsCallback(removed);
        scheduler.addTimingsCallback(added);
      }
    }

    scheduler.addTimingsCallback(first);
    scheduler.addTimingsCallback(removed);
    scheduler.addTimingsCallback(last);
    try {
      scheduler.platformDispatcher.onReportTimings!(<FrameTiming>[]);
      expect(seen, ['first', 'last']);
      seen.clear();
      scheduler.platformDispatcher.onReportTimings!(<FrameTiming>[]);
      expect(seen, ['first', 'last', 'new']);
    } finally {
      scheduler.removeTimingsCallback(first);
      scheduler.removeTimingsCallback(last);
      scheduler.removeTimingsCallback(firstBatch ? removed : added);
    }
  });
}
