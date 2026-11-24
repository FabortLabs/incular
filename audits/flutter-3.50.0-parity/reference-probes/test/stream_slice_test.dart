import 'dart:async';

import 'package:flutter_test/flutter_test.dart';

void main() {
  test('ordinary broadcast and close control', () async {
    final c = StreamController<int>.broadcast(sync: true);
    final a = <int>[], b = <int>[];
    var done = 0;
    c.stream.listen(a.add, onDone: () => done++);
    c.stream.listen(b.add, onDone: () => done++);
    c.add(1);
    c.add(2);
    await c.close();
    expect(a, [1, 2]);
    expect(b, [1, 2]);
    expect(done, 2);
  });
  test('cancellation skips a listener whose turn has not arrived', () async {
    final c = StreamController<int>.broadcast(sync: true);
    final b = <int>[];
    late StreamSubscription<int> sub;
    c.stream.listen((_) {
      sub.cancel();
    });
    sub = c.stream.listen(b.add);
    c.add(1);
    expect(b, isEmpty);
    await c.close();
  });
  test('same-controller reentrant add does not overtake', () async {
    final c = StreamController<int>.broadcast(sync: true);
    final b = <int>[];
    var rejected = false;
    c.stream.listen((v) {
      if (v == 1) {
        try {
          c.add(2);
        } on StateError {
          rejected = true;
        }
      }
    });
    c.stream.listen(b.add);
    c.add(1);
    expect(rejected, isTrue);
    expect(b, [1]);
    await c.close();
  });
  test('Done is terminal across listeners', () async {
    final c = StreamController<int>.broadcast(sync: true);
    final a = <String>[], b = <String>[];
    var rejected = false;
    c.stream.listen(
      (v) => a.add('data$v'),
      onDone: () {
        a.add('Done');
        try {
          c.add(2);
        } on StateError {
          rejected = true;
        }
      },
    );
    c.stream.listen((v) => b.add('data$v'), onDone: () => b.add('Done'));
    await c.close();
    expect(rejected, isTrue);
    expect(a, ['Done']);
    expect(b, ['Done']);
  });
}
