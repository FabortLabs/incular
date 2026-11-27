import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'Planar pointer perspective preserves globals and maps both ends of delta',
    () {
      final paint = Matrix4.identity()
        ..setEntry(0, 0, 2)
        ..setEntry(1, 1, 3)
        ..setEntry(2, 2, 9)
        ..setEntry(3, 0, 0.01)
        ..setEntry(0, 2, 7)
        ..setEntry(1, 2, 8)
        ..setEntry(3, 2, 10)
        ..setEntry(0, 3, 10)
        ..setEntry(1, 3, 20)
        ..setEntry(2, 3, 30);
      final pointer = PointerEvent.removePerspectiveTransform(paint);
      expect(pointer.entry(3, 0), 0.01);
      expect(pointer.entry(2, 2), 1);
      expect(pointer.entry(3, 2), 0);
      expect(pointer.entry(2, 3), 0);
      expect(paint.entry(2, 2), 9);

      const original = PointerMoveEvent(
        pointer: 34,
        kind: PointerDeviceKind.stylus,
        device: 10,
        position: Offset(20, 30),
        delta: Offset(5, 5),
        buttons: kPrimaryButton,
        pressure: 0.4,
      );
      final event = original.transformed(pointer);
      expect(event.position, original.position);
      expect(event.delta, original.delta);
      expect(event.device, original.device);
      expect(event.pressure, original.pressure);
      expect(event.localPosition.dx, closeTo(50 / 1.2, 1e-9));
      expect(event.localPosition.dy, closeTo(110 / 1.2, 1e-9));
      expect(event.localDelta.dx, closeTo(50 / 1.2 - 40 / 1.15, 1e-9));
      expect(event.localDelta.dy, closeTo(110 / 1.2 - 95 / 1.15, 1e-9));
    },
  );
}
