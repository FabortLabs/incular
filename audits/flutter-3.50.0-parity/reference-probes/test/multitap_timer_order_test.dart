// Authored for Incular parity; not an upstream test registration.

import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('long tap timers use deadline and registration order', (
    WidgetTester tester,
  ) async {
    final tap = MultiTapGestureRecognizer(longTapDelay: kLongPressTimeout);
    addTearDown(tap.dispose);
    final longTaps = <(int, Offset)>[];
    tap.onLongTapDown = (pointer, details) =>
        longTaps.add((pointer, details.globalPosition));

    void down(int id) {
      final pointer = TestPointer(id);
      final event = pointer.down(Offset(id.toDouble(), 0));
      tap.addPointer(event);
      GestureBinding.instance.gestureArena.close(id);
      GestureBinding.instance.pointerRouter.route(event);
    }

    down(40);
    down(7);
    await tester.pump(const Duration(milliseconds: 16));
    down(2);
    await tester.pump(const Duration(milliseconds: 504));
    expect(longTaps, [
      (40, const Offset(40, 0)),
      (7, const Offset(7, 0)),
      (2, const Offset(2, 0)),
    ]);
    await tester.pump(const Duration(milliseconds: 280));
    expect(longTaps, hasLength(3));
  });
}
