import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

// Incular stores the result in f32, but Flutter does not narrow the host
// coordinates or ratio before the divide.
int f32Bits(double value) {
  final bytes = ByteData(4)..setFloat32(0, value);
  return bytes.getUint32(0);
}

void main() {
  test(
    'pointer and scroll narrow only after dividing by a fractional view ratio',
    () {
      final List<PointerEvent> events = PointerEventConverter.expand(
        <ui.PointerData>[
          const ui.PointerData(
            change: ui.PointerChange.hover,
            kind: ui.PointerDeviceKind.mouse,
            physicalX: 10,
            physicalY: -10.0000003,
          ),
          const ui.PointerData(
            change: ui.PointerChange.hover,
            kind: ui.PointerDeviceKind.mouse,
            signalKind: ui.PointerSignalKind.scroll,
            scrollDeltaX: 10,
            scrollDeltaY: -10.0000003,
          ),
        ],
        (int viewId) => 1.3,
      ).toList();
      expect(events, hasLength(2));
      expect(f32Bits(events.first.position.dx), 0x40f62762);
      expect(f32Bits(events.first.position.dy), 0xc0f62763);
      final scroll = events.last as PointerScrollEvent;
      expect(f32Bits(scroll.scrollDelta.dx), 0x40f62762);
      expect(f32Bits(scroll.scrollDelta.dy), 0xc0f62763);
    },
  );
}
