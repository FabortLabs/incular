import 'dart:async';

import 'package:fake_async/fake_async.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('list markers indent continuations and stack lines bypass wrapping', () {
    expect(debugWordWrap('  2. Alpha beta gamma', 13), [
      '  2. Alpha',
      '     beta',
      '     gamma',
    ]);
    expect(debugWordWrap('    #12 file.dart:20', 2), ['    #12 file.dart:20']);
    expect(debugWordWrap('foo extremelylongword', 3), [
      'foo',
      'extremelylongword',
    ]);
    expect(debugWordWrap('😀 😀', 4), ['😀', '😀']);
  });

  test('UTF-16 budget defers a pair of messages until the same timer', () {
    FakeAsync().run((clock) {
      final sizes = <int>[];
      runZoned(
        () {
          debugPrintThrottled('😀' * (6 * 1024));
          debugPrintThrottled('X');
          expect(clock.pendingTimers, hasLength(1));
          clock.elapse(const Duration(milliseconds: 2));
          debugPrintThrottled('Y');
          expect(clock.pendingTimers, hasLength(1));
          expect(sizes, [12 * 1024]);
          clock.elapse(const Duration(seconds: 1));
          expect(sizes, [12 * 1024, 1, 1]);
        },
        zoneSpecification: ZoneSpecification(
          print: (self, parent, zone, line) => sizes.add(line.length),
        ),
      );
    });
  });
}
