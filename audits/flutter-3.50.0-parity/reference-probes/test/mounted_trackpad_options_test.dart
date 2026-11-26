import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'trackpad scroll scale anchors its focal point and uses configured gain',
    (tester) async {
      final scales = <ScaleUpdateDetails>[];
      await tester.pumpWidget(
        GestureDetector(
          behavior: HitTestBehavior.opaque,
          trackpadScrollCausesScale: true,
          trackpadScrollToScaleFactor: const Offset(1 / 125, 0),
          onScaleUpdate: scales.add,
          child: const SizedBox.expand(),
        ),
      );
      const origin = Offset(30, 30);
      final gesture = await tester.startGesture(
        origin,
        kind: PointerDeviceKind.trackpad,
      );
      expect(scales, isEmpty);
      await gesture.panZoomUpdate(origin, pan: const Offset(125, 200));
      expect(scales.single.scale, closeTo(math.e, 0.0001));
      expect(scales.single.focalPoint, origin);
      expect(scales.single.focalPointDelta, Offset.zero);
      await gesture.panZoomEnd();
      expect(scales, hasLength(1));
    },
  );

  testWidgets('trackpad scale bypasses supported devices but drag does not', (
    tester,
  ) async {
    final events = <String>[];
    await tester.pumpWidget(
      GestureDetector(
        behavior: HitTestBehavior.opaque,
        supportedDevices: const {PointerDeviceKind.touch},
        onHorizontalDragDown: (_) => events.add('drag-down'),
        onHorizontalDragCancel: () => events.add('drag-cancel'),
        onScaleStart: (_) => events.add('scale-start'),
        onScaleUpdate: (_) => events.add('scale-update'),
        onScaleEnd: (_) => events.add('scale-end'),
        child: const SizedBox.expand(),
      ),
    );
    const origin = Offset(30, 30);
    final gesture = await tester.startGesture(
      origin,
      kind: PointerDeviceKind.trackpad,
    );
    expect(events, ['scale-start']);
    await gesture.panZoomUpdate(origin, scale: 2);
    await gesture.panZoomEnd();
    expect(events, ['scale-start', 'scale-update', 'scale-end']);
  });

  testWidgets('a trackpad drag cancels once if scale wins after drag-down', (
    tester,
  ) async {
    final events = <String>[];
    await tester.pumpWidget(
      GestureDetector(
        behavior: HitTestBehavior.opaque,
        onHorizontalDragDown: (details) {
          expect(details.globalPosition, const Offset(30, 30));
          events.add('drag-down');
        },
        onHorizontalDragStart: (_) => events.add('drag-start'),
        onHorizontalDragCancel: () => events.add('drag-cancel'),
        onScaleStart: (_) => events.add('scale-start'),
        onScaleUpdate: (_) => events.add('scale-update'),
        onScaleEnd: (_) => events.add('scale-end'),
        child: const SizedBox.expand(),
      ),
    );
    const origin = Offset(30, 30);
    final gesture = await tester.startGesture(
      origin,
      kind: PointerDeviceKind.trackpad,
    );
    expect(events, ['drag-down']);
    await gesture.panZoomUpdate(origin, scale: 2);
    expect(events, ['drag-down', 'drag-cancel', 'scale-start', 'scale-update']);
    await gesture.panZoomEnd();
    expect(events, [
      'drag-down',
      'drag-cancel',
      'scale-start',
      'scale-update',
      'scale-end',
    ]);
  });
}
