// Authored for Incular parity; not an upstream test registration.

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class LazyListeners
    with
        AnimationLazyListenerMixin,
        AnimationLocalListenersMixin,
        AnimationLocalStatusListenersMixin {
  int starts = 0;
  int stops = 0;

  @override
  void didStartListening() => starts++;

  @override
  void didStopListening() => stops++;

  void values() => notifyListeners();
  void statuses(AnimationStatus status) => notifyStatusListeners(status);
}

void main() {
  test(
    'value and status duplicates share the lazy binding but notify differently',
    () {
      final listeners = LazyListeners();
      var values = 0;
      final statuses = <AnimationStatus>[];
      void onValue() => values++;
      void onStatus(AnimationStatus status) => statuses.add(status);
      void fakeValue() {}
      void fakeStatus(AnimationStatus status) {}

      listeners.addListener(onValue);
      listeners.addListener(onValue);
      listeners.addStatusListener(onStatus);
      listeners.addStatusListener(onStatus);
      expect((listeners.starts, listeners.stops), (1, 0));
      listeners.removeListener(fakeValue);
      listeners.removeStatusListener(fakeStatus);
      expect((listeners.starts, listeners.stops), (1, 0));

      listeners.values();
      listeners.statuses(AnimationStatus.forward);
      expect(values, 1);
      expect(statuses, [AnimationStatus.forward, AnimationStatus.forward]);
      statuses.clear();

      listeners.removeListener(onValue);
      listeners.removeStatusListener(onStatus);
      listeners.values();
      listeners.statuses(AnimationStatus.completed);
      expect(values, 2);
      expect(statuses, [AnimationStatus.completed]);
      listeners.removeListener(onValue);
      expect(listeners.isListening, isTrue);
      expect(listeners.stops, 0);
      listeners.removeStatusListener(onStatus);
      listeners.removeStatusListener(onStatus);
      expect(listeners.isListening, isFalse);
      expect((listeners.starts, listeners.stops), (1, 1));

      listeners.addStatusListener(onStatus);
      listeners.removeStatusListener(onStatus);
      expect((listeners.starts, listeners.stops), (2, 2));
    },
  );

  test('removals suppress later callbacks and additions wait for the next dispatch', () {
    final listeners = LazyListeners();
    final events = <String>[];
    void old() => events.add('old');
    void next() => events.add('new');
    var replaced = false;
    listeners.addListener(() {
      events.add('first');
      if (!replaced) {
        replaced = true;
        listeners.removeListener(old);
        listeners.addListener(next);
      }
    });
    listeners.addListener(old);
    listeners.values();
    expect(events, ['first']);
    events.clear();
    listeners.values();
    expect(events, ['first', 'new']);
    expect(listeners.stops, 0);
  });

  test(
    'listener errors are reported without losing the remaining snapshot',
    () {
      final listeners = LazyListeners();
      final notes = <Object>[];
      final old = FlutterError.onError;
      FlutterError.onError = (details) => notes.add(details.exception);
      addTearDown(() => FlutterError.onError = old);
      listeners.addStatusListener((_) => throw 7);
      final statuses = <AnimationStatus>[];
      listeners.addStatusListener(statuses.add);
      listeners.statuses(AnimationStatus.reverse);
      expect(notes, [7]);
      expect(statuses, [AnimationStatus.reverse]);
    },
  );
}
