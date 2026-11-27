import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  setUp(() {
    FlutterTimeline.debugCollectionEnabled = true;
  });

  tearDown(() {
    FlutterTimeline.debugCollectionEnabled = false;
  });

  test('reset drops finished blocks but preserves active nested timings', () {
    FlutterTimeline.timeSync('outside', () {
      FlutterTimeline.timeSync('discarded', () {});
      FlutterTimeline.debugReset();
      FlutterTimeline.timeSync('inside', () {});
    });
    final blocks = FlutterTimeline.debugCollect().timedBlocks;
    expect(blocks.map((block) => block.name), ['inside', 'outside']);
    expect(blocks[1].start, lessThanOrEqualTo(blocks[0].start));
    expect(blocks[1].end, greaterThanOrEqualTo(blocks[0].end));
  });

  test('timeSync includes a block that throws and preserves its exception', () {
    expect(
      () => FlutterTimeline.timeSync('throw', () => throw StateError('sample')),
      throwsStateError,
    );
    expect(FlutterTimeline.debugCollect().getAggregated('throw').count, 1);
  });
}
