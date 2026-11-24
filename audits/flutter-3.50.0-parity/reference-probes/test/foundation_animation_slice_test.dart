import 'dart:ui' show Size;

import 'package:flutter/animation.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('notifier remove before turn, add waits for next notification', () {
    final n = ChangeNotifier(), calls = <String>[];
    void b() => calls.add('B');
    void c() => calls.add('C');
    var first = true;
    n.addListener(() {
      calls.add('A');
      if (first) {
        first = false;
        n.removeListener(b);
        n.addListener(c);
      }
    });
    n.addListener(b);
    n.notifyListeners();
    expect(calls, ['A']);
    n.notifyListeners();
    expect(calls, ['A', 'A', 'C']);
    n.dispose();
  });
  test('nullable size tween preserves the missing endpoint', () {
    final tween = SizeTween(end: const Size(10, 20));
    expect(tween.transform(.5), const Size(5, 10));
    expect(tween.transform(1), const Size(10, 20));
    expect(tween.transform(0), isNull);
  });
  test('integer tween keeps midpoints around the f32 precision boundary', () {
    final ordinary = IntTween(begin: 2, end: 4);
    expect(ordinary.transform(.5), 3);
    final tween = IntTween(begin: 16777217, end: 16777219);
    expect(tween.transform(.5), 16777218);
  });
}
