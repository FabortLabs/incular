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
    await h.mount(
      (_) => Center(
        child: h.probe(
          'target',
          GestureDetector(
            onTap: () => taps++,
            onTapDown: (details) => downY = details.globalPosition.dy,
            child: const SizedBox(
              width: 120,
              height: 80,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.down(const Offset(400, 300));
    await h.up();
    await h.frames(1);
  });

  frameScenario('gestures__double_tap', (h) async {
    var doubleTaps = 0;
    h.value('double_tap.count', () => doubleTaps.toDouble());
    await h.mount(
      (_) => Center(
        child: h.probe(
          'target',
          GestureDetector(
            onDoubleTap: () => doubleTaps++,
            child: const SizedBox(
              width: 120,
              height: 80,
              child: ColoredBox(color: green),
            ),
          ),
        ),
      ),
    );
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
    await h.mount(
      (_) => Center(
        child: h.probe(
          'target',
          GestureDetector(
            onLongPress: () => longPresses++,
            onLongPressStart: (details) => startY = details.globalPosition.dy,
            child: const SizedBox(
              width: 120,
              height: 80,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
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
    await h.mount(
      (_) => Center(
        child: h.probe(
          'target',
          GestureDetector(
            dragStartBehavior: DragStartBehavior.down,
            onTap: () {},
            onVerticalDragStart: (details) =>
                startY = details.globalPosition.dy,
            onVerticalDragUpdate: (details) => deltaY += details.delta.dy,
            onVerticalDragEnd: (details) =>
                endVelocityY = details.velocity.pixelsPerSecond.dy,
            child: const SizedBox(
              width: 160,
              height: 120,
              child: ColoredBox(color: green),
            ),
          ),
        ),
      ),
    );
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
    await h.mount(
      (_) => Center(
        child: h.probe(
          'target',
          GestureDetector(
            onScaleStart: (_) => starts++,
            onScaleUpdate: (details) {
              updates++;
              focalY = details.focalPoint.dy;
            },
            child: const SizedBox(
              width: 200,
              height: 160,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
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
    await h.mount(
      (_) => Center(
        child: h.probe(
          'target',
          GestureDetector(
            onScaleUpdate: (details) {
              scale = details.scale;
              pointers = details.pointerCount.toDouble();
            },
            child: const SizedBox(
              width: 200,
              height: 160,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
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

  frameScenario('gestures__draggable_drop_target', (h) async {
    var dragStarts = 0;
    var acceptedData = 0;
    h.value('drag.started', () => dragStarts.toDouble());
    h.value('target.accepted_data', () => acceptedData.toDouble());
    await h.mount(
      (_) => Overlay(
        initialEntries: <OverlayEntry>[
          OverlayEntry(
            builder: (_) => Stack(
              children: <Widget>[
                Positioned(
                  left: 500,
                  top: 250,
                  child: DragTarget<int>(
                    onAcceptWithDetails: (details) =>
                        acceptedData = details.data,
                    builder: (context, candidates, rejected) => h.probe(
                      'drop_target',
                      const SizedBox(
                        width: 120,
                        height: 80,
                        child: ColoredBox(color: green),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  left: 100,
                  top: 250,
                  child: h.probe(
                    'drag_source',
                    Draggable<int>(
                      data: 7,
                      feedback: const SizedBox(
                        width: 40,
                        height: 40,
                        child: ColoredBox(color: blue),
                      ),
                      onDragStarted: () => dragStarts += 1,
                      child: const SizedBox(
                        width: 80,
                        height: 60,
                        child: ColoredBox(color: blue),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
    await h.frames(1);
    await h.down(h.center('drag_source'));
    await h.frames(1);
    await h.move(const Offset(160, 280));
    await h.frames(1);
    await h.move(const Offset(560, 290));
    await h.frames(1);
    await h.up();
    await h.frames(1);
  });

  frameScenario('gestures__long_press_draggable_quick_move', (h) async {
    var dragStarts = 0;
    h.value('drag.started', () => dragStarts.toDouble());
    await h.mount(
      (_) => Overlay(
        initialEntries: <OverlayEntry>[
          OverlayEntry(
            builder: (_) => Center(
              child: h.probe(
                'drag_source',
                LongPressDraggable<int>(
                  data: 7,
                  feedback: const SizedBox(
                    width: 40,
                    height: 40,
                    child: ColoredBox(color: blue),
                  ),
                  onDragStarted: () => dragStarts += 1,
                  child: const SizedBox(
                    width: 120,
                    height: 80,
                    child: ColoredBox(color: blue),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
    await h.frames(1);
    final start = h.center('drag_source');
    await h.down(start);
    await h.pump(const Duration(milliseconds: 100));
    await h.move(start + const Offset(40, 0));
    await h.frames(1);
    await h.up();
    await h.frames(1);
  });

  frameScenario('gestures__dismissible_threshold', (h) async {
    var progress = 0.0;
    var reached = 0;
    var dismissed = 0;
    h.value('dismiss.progress', () => progress);
    h.value('dismiss.reached', () => reached.toDouble());
    h.value('dismiss.count', () => dismissed.toDouble());
    await h.mount(
      (_) => Center(
        child: Dismissible(
          key: const ValueKey<String>('dismissible'),
          onDismissed: (_) => dismissed += 1,
          onUpdate: (details) {
            progress = details.progress;
            reached = details.reached ? 1 : 0;
          },
          child: h.probe(
            'dismissible',
            const SizedBox(
              width: 160,
              height: 80,
              child: ColoredBox(color: green),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    final start = h.center('dismissible');
    await h.down(start);
    await h.frames(1);
    await h.move(start + const Offset(100, 0));
    await h.frames(1);
    await h.up();
    await h.pump(const Duration(milliseconds: 600));
  });

  frameScenario('gestures__interactive_viewer_mouse_wheel_zoom', (h) async {
    final controller = TransformationController();
    h.value('viewer.scale', () => controller.value.getMaxScaleOnAxis());
    h.value('viewer.translate_x', () => controller.value.getTranslation().x);
    h.value('viewer.translate_y', () => controller.value.getTranslation().y);
    await h.mount(
      (_) => Center(
        child: h.probe(
          'viewer',
          SizedBox(
            width: 200,
            height: 160,
            child: InteractiveViewer(
              constrained: false,
              transformationController: controller,
              child: const SizedBox(
                width: 400,
                height: 320,
                child: ColoredBox(color: blue),
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.wheel(const Offset(400, 300), const Offset(0, -40));
    await h.frames(1);
    controller.dispose();
  });

  frameScenario('gestures__reorderable_list_long_press', (h) async {
    var startedIndex = -1;
    var endedIndex = -1;
    var reorderFrom = -1;
    var reorderTo = -1;
    h.value('reorder.started_index', () => startedIndex.toDouble());
    h.value('reorder.ended_index', () => endedIndex.toDouble());
    h.value('reorder.from', () => reorderFrom.toDouble());
    h.value('reorder.to', () => reorderTo.toDouble());
    await h.mount(
      (_) => WidgetsApp(
        color: blue,
        pageRouteBuilder: <T>(settings, builder) => PageRouteBuilder<T>(
          settings: settings,
          pageBuilder: (context, animation, secondaryAnimation) =>
              builder(context),
        ),
        home: SizedBox(
          width: 180,
          height: 240,
          child: ReorderableList(
            itemCount: 3,
            itemExtent: 60,
            onReorder: (oldIndex, newIndex) {
              reorderFrom = oldIndex;
              reorderTo = newIndex;
            },
            onReorderStart: (index) => startedIndex = index,
            onReorderEnd: (index) => endedIndex = index,
            itemBuilder: (context, index) =>
                ReorderableDelayedDragStartListener(
                  key: ValueKey<int>(index),
                  index: index,
                  child: h.probe(
                    'reorder_row_$index',
                    SizedBox(
                      width: 180,
                      height: 60,
                      child: ColoredBox(color: index.isEven ? blue : green),
                    ),
                  ),
                ),
          ),
        ),
      ),
    );
    await h.frames(1);
    final start = h.center('reorder_row_0');
    await h.down(start);
    await h.pump(const Duration(milliseconds: 600));
    await h.move(start + const Offset(0, 150));
    await h.frames(2);
    await h.up();
    await h.frames(1);
  });
}
