// Gesture frame scenarios. Each one has a twin with the same name in
// harness/incular/tests/frames/gestures.rs; keep them identical.

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color blue = Color(0xFF2196F3);
const Color green = Color(0xFF4CAF50);

void main() {
  frameScenario('gestures__tap', (h) async {
    var taps = 0;
    var downY = 0.0;
    h.value('tap.count', () => taps.toDouble());
    h.value('tap.down_y', () => downY);
    await h.mount((_) => Center(
          child: h.probe(
            'target',
            GestureDetector(
              onTap: () => taps++,
              onTapDown: (details) => downY = details.globalPosition.dy,
              child: const SizedBox(width: 120, height: 80, child: ColoredBox(color: blue)),
            ),
          ),
        ));
    await h.frames(1);
    await h.down(const Offset(400, 300));
    await h.up();
    await h.frames(1);
  });

  frameScenario('gestures__double_tap', (h) async {
    var doubleTaps = 0;
    h.value('double_tap.count', () => doubleTaps.toDouble());
    await h.mount((_) => Center(
          child: h.probe(
            'target',
            GestureDetector(
              onDoubleTap: () => doubleTaps++,
              child: const SizedBox(width: 120, height: 80, child: ColoredBox(color: green)),
            ),
          ),
        ));
    await h.frames(1);
    await h.down(const Offset(400, 300));
    await h.up();
    await h.frames(3);
    await h.down(const Offset(400, 300));
    await h.up();
    await h.frames(4);
  });

  frameScenario('gestures__long_press', (h) async {
    var longPresses = 0;
    var startY = 0.0;
    h.value('long_press.count', () => longPresses.toDouble());
    h.value('long_press.start_y', () => startY);
    await h.mount((_) => Center(
          child: h.probe(
            'target',
            GestureDetector(
              onLongPress: () => longPresses++,
              onLongPressStart: (details) => startY = details.globalPosition.dy,
              child: const SizedBox(width: 120, height: 80, child: ColoredBox(color: blue)),
            ),
          ),
        ));
    await h.frames(1);
    await h.down(const Offset(400, 300));
    await h.frames(32);
    await h.up();
    await h.frames(1);
  });

  frameScenario('gestures__vertical_drag_down', (h) async {
    var startY = 0.0;
    var deltaY = 0.0;
    var endVelocityY = 0.0;
    h.value('drag.start_y', () => startY);
    h.value('drag.delta_y', () => deltaY);
    h.value('drag.end_velocity_y', () => endVelocityY);
    await h.mount((_) => Center(
          child: h.probe(
            'target',
            GestureDetector(
              dragStartBehavior: DragStartBehavior.down,
              onTap: () {},
              onVerticalDragStart: (details) => startY = details.globalPosition.dy,
              onVerticalDragUpdate: (details) => deltaY += details.delta.dy,
              onVerticalDragEnd: (details) => endVelocityY = details.velocity.pixelsPerSecond.dy,
              child: const SizedBox(width: 160, height: 120, child: ColoredBox(color: green)),
            ),
          ),
        ));
    await h.frames(1);
    await h.down(const Offset(400, 300));
    await h.frames(1);
    await h.move(const Offset(400, 290));
    await h.frames(1);
    await h.move(const Offset(400, 280));
    await h.frames(1);
    await h.move(const Offset(400, 250));
    await h.frames(1);
    await h.up();
    await h.frames(1);
  });

  frameScenario('gestures__scale_single_pointer_pan', (h) async {
    var starts = 0;
    var updates = 0;
    var focalY = 0.0;
    h.value('scale.starts', () => starts.toDouble());
    h.value('scale.updates', () => updates.toDouble());
    h.value('scale.focal_y', () => focalY);
    await h.mount((_) => Center(
          child: h.probe(
            'target',
            GestureDetector(
              onScaleStart: (_) => starts++,
              onScaleUpdate: (details) {
                updates++;
                focalY = details.focalPoint.dy;
              },
              child: const SizedBox(width: 200, height: 160, child: ColoredBox(color: blue)),
            ),
          ),
        ));
    await h.frames(1);
    await h.down(const Offset(400, 300));
    await h.frames(1);
    await h.move(const Offset(400, 250));
    await h.frames(1);
    await h.move(const Offset(400, 220));
    await h.frames(1);
    await h.up();
    await h.frames(1);
  });

  frameScenario('gestures__scale__two_pointer_pinch', (h) async {
    var scale = 1.0;
    var pointers = 0.0;
    h.value('scale', () => scale);
    h.value('pointers', () => pointers);
    await h.mount((_) => Center(
          child: h.probe(
            'target',
            GestureDetector(
              onScaleUpdate: (details) {
                scale = details.scale;
                pointers = details.pointerCount.toDouble();
              },
              child: const SizedBox(width: 200, height: 160, child: ColoredBox(color: blue)),
            ),
          ),
        ));
    await h.down(const Offset(370, 300));
    await h.frames(1);
    await h.secondDown(const Offset(430, 300));
    await h.frames(1);
    await h.secondMove(const Offset(460, 300));
    await h.frames(1);
    await h.move(const Offset(350, 300));
    await h.frames(1);
    await h.secondUp();
    await h.frames(1);
    await h.up();
    await h.frames(1);
  });
}
