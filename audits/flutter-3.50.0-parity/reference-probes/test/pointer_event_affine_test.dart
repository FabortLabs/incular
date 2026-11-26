import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a large affine translation does not erase a one-pixel local move', () {
    final transform = Matrix4.translationValues(33554432, -33554432, 0);
    const event = PointerMoveEvent(
      position: Offset(20, 30),
      delta: Offset(1, -1),
    );
    final local = event.transformed(transform);
    expect(local.localPosition, const Offset(33554452, -33554402));
    expect(local.localDelta, const Offset(1, -1));
    expect(local.position, event.position);
    expect(local.delta, event.delta);
  });

  test('trackpad uses device touch slop while a mouse remains precise', () {
    const settings = DeviceGestureSettings(touchSlop: 7);
    expect(computeHitSlop(PointerDeviceKind.mouse, settings), 1);
    expect(computePanSlop(PointerDeviceKind.mouse, settings), 2);
    expect(computeHitSlop(PointerDeviceKind.trackpad, settings), 7);
    expect(computePanSlop(PointerDeviceKind.trackpad, settings), 14);
    expect(computeScaleSlop(PointerDeviceKind.trackpad), 18);
  });
}
