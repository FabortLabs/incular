import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('physical pan zoom keeps the captured listener transform', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 2.5;
    tester.view.physicalSize = const Size(500, 500);
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final events = <PointerEvent>[];
    await tester.pumpWidget(
      Transform.scale(
        scale: 2,
        child: Listener(
          behavior: HitTestBehavior.opaque,
          onPointerPanZoomStart: events.add,
          onPointerPanZoomUpdate: events.add,
          onPointerPanZoomEnd: events.add,
          child: const SizedBox.expand(),
        ),
      ),
    );

    final viewId = tester.view.viewId;
    final start = ui.PointerData(
      viewId: viewId,
      pointerIdentifier: 11,
      device: 25,
      kind: ui.PointerDeviceKind.trackpad,
      change: ui.PointerChange.panZoomStart,
      physicalX: 250,
      physicalY: 250,
    );
    final update = ui.PointerData(
      viewId: viewId,
      pointerIdentifier: 11,
      device: 25,
      kind: ui.PointerDeviceKind.trackpad,
      change: ui.PointerChange.panZoomUpdate,
      physicalX: 1400,
      physicalY: 1500,
      panX: 40,
      panY: 20,
      panDeltaX: 30,
      panDeltaY: -10,
      scale: 1.2,
      rotation: 0.23,
      synthesized: true,
    );
    final end = ui.PointerData(
      viewId: viewId,
      pointerIdentifier: 11,
      device: 25,
      kind: ui.PointerDeviceKind.trackpad,
      change: ui.PointerChange.panZoomEnd,
      physicalX: 1400,
      physicalY: 1500,
    );
    tester.binding.platformDispatcher.onPointerDataPacket!(
      ui.PointerDataPacket(data: [update]),
    );
    expect(events, isEmpty);
    tester.binding.platformDispatcher.onPointerDataPacket!(
      ui.PointerDataPacket(data: [start, update, end, update, end]),
    );
    expect(events, hasLength(3));
    expect(events.first, isA<PointerPanZoomStartEvent>());
    expect(events.last, isA<PointerPanZoomEndEvent>());
    final movement = events[1] as PointerPanZoomUpdateEvent;
    expect(events.first.localPosition, const Offset(100, 100));
    expect(movement.position, const Offset(560, 600));
    expect(movement.localPosition, const Offset(330, 350));
    expect(movement.pan, const Offset(16, 8));
    expect(movement.localPan, const Offset(58, 54));
    expect(movement.panDelta, const Offset(12, -4));
    expect(movement.localPanDelta, const Offset(6, -2));
    expect(movement.scale, 1.2);
    expect(movement.rotation, 0.23);
    expect(movement.synthesized, isTrue);
    expect(
      events.every((sample) => sample.pointer == 11 && sample.device == 25),
      isTrue,
    );
  });
}
