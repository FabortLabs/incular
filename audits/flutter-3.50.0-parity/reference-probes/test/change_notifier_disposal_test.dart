import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

class Notifier extends ChangeNotifier {
  void notify() => notifyListeners();
}

void main() {
  test('disposing during notification does not dispose the notifier', () {
    final notifier = Notifier();
    var called = 0;
    notifier.addListener(() {
      expect(notifier.dispose, throwsA(isA<AssertionError>()));
    });
    notifier.addListener(() => called++);
    notifier.notify();
    expect(called, 1);
    notifier.dispose();
    notifier.removeListener(() {});
  });

  test(
    'nested dispatch sees the new boundary but the outer dispatch does not',
    () {
      final notifier = Notifier();
      final log = <String>[];
      var first = true;
      void old() => log.add('B');
      notifier.addListener(() {
        log.add('A');
        if (first) {
          first = false;
          notifier.removeListener(old);
          notifier.addListener(() => log.add('D'));
          notifier.notify();
        }
      });
      notifier.addListener(old);
      notifier.addListener(() => log.add('C'));
      notifier.notify();
      expect(log, ['A', 'A', 'C', 'D', 'C']);
      log.clear();
      notifier.notify();
      expect(log, ['A', 'C', 'D']);
      notifier.dispose();
    },
  );
}
