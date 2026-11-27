import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a small measured pan delta survives projection at a large cumulative pan', () {
    final PointerPanZoomUpdateEvent event = const PointerPanZoomUpdateEvent(
      pan: Offset(33554432, 33554432),
      panDelta: Offset(0.5, -0.5),
      position: Offset(560, 600),
    ).transformed(Matrix4.translationValues(50, 50, 0)..scale(0.5, 0.5, 1.0));

    expect(event.localPosition, const Offset(330, 350));
    expect(event.localPan, const Offset(16777266, 16777266));
    expect(event.panDelta, const Offset(0.5, -0.5));
    expect(event.localPanDelta, const Offset(0.25, -0.25));
  });
}
