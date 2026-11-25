import 'package:flutter/animation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test('a train hop publishes status, value, then the switch callback', () {
    AnimationController controller(double value) {
      final node = AnimationController(
        value: value,
        duration: const Duration(milliseconds: 100),
        vsync: const TestVSync(),
      );
      addTearDown(node.dispose);
      return node;
    }

    final first = controller(0.5)..value = 0.5;
    final next = controller(0.75)..reverse();
    final log = <Object>[];
    final train = TrainHoppingAnimation(
      first,
      next,
      onSwitchedTrain: () => log.add('hop'),
    );
    addTearDown(train.dispose);
    train.addListener(() => log.add(train.value));
    train.addStatusListener(log.add);

    first.value = 0.55;
    expect(log, [0.55]);
    log.clear();
    next.value = 0.8;
    expect(log, isEmpty);
    next.value = 0.25;
    expect(log, [AnimationStatus.reverse, 0.25, 'hop']);
    log.clear();
    first.value = 0.6;
    expect(log, isEmpty);
    next.value = 0.2;
    expect(log, [0.2]);
  });
}
