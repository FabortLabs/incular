import 'package:flutter/animation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test('Proxy parent swaps notify only changed values and statuses', () {
    AnimationController controller(double value) {
      final controller = AnimationController(
        vsync: const TestVSync(),
        value: value,
        duration: const Duration(milliseconds: 100),
      );
      addTearDown(controller.dispose);
      return controller;
    }

    final first = controller(0.5);
    final equal = controller(0.5);
    final backwards = controller(0.5)..reverse();
    final changed = controller(0.75)..reverse();
    final proxy = ProxyAnimation(first);
    final values = <double>[];
    final statuses = <AnimationStatus>[];
    void onValue() => values.add(proxy.value);
    proxy.addListener(onValue);
    proxy.addStatusListener(statuses.add);
    addTearDown(() {
      proxy.removeListener(onValue);
      proxy.removeStatusListener(statuses.add);
    });

    final snapshots = <List<Object>>[];
    void capture() {
      snapshots.add([
        proxy.value,
        proxy.status,
        values.toList(),
        statuses.toList(),
      ]);
      values.clear();
      statuses.clear();
    }

    proxy.parent = first; // identical parent
    capture();
    proxy.parent = equal; // different parent, same value and status
    capture();
    proxy.parent = backwards; // status only
    capture();
    proxy.parent = changed; // value only
    capture();
    proxy.parent = null; // retain both; notify neither
    capture();
    changed.value = 0.25; // old parent no longer notifies
    capture();

    const forward = AnimationStatus.forward;
    const reverse = AnimationStatus.reverse;
    expect(snapshots, [
      [0.5, forward, [], []],
      [0.5, forward, [], []],
      [
        0.5,
        reverse,
        [],
        [reverse],
      ],
      [
        0.75,
        reverse,
        [0.75],
        [],
      ],
      [0.75, reverse, [], []],
      [0.75, reverse, [], []],
    ]);
  });
}
