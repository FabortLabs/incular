// Sample frame scenarios. Each one has a twin with the same name in
// harness/incular/tests/frames/samples.rs; keep them identical.

import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color blue = Color(0xFF2196F3);
const Color orange = Color(0xFFFF5722);
const Color green = Color(0xFF4CAF50);

void main() {
  frameScenario('sample_animated_container', (h) async {
    await h.mount((phase) => Center(
          child: h.probe(
            'box',
            AnimatedContainer(
              duration: const Duration(milliseconds: 300),
              curve: Curves.easeInOut,
              width: phase == 0 ? 100 : 200,
              height: phase == 0 ? 100 : 60,
              color: phase == 0 ? blue : orange,
            ),
          ),
        ));
    await h.frames(2);
    h.setPhase(1);
    await h.frames(25);
  });

  frameScenario('sample_animated_opacity', (h) async {
    await h.mount((phase) => Center(
          child: AnimatedOpacity(
            opacity: phase == 0 ? 1 : 0.2,
            duration: const Duration(milliseconds: 200),
            child: h.probe(
              'box',
              const SizedBox(width: 80, height: 80, child: ColoredBox(color: green)),
            ),
          ),
        ));
    await h.frames(2);
    h.setPhase(1);
    await h.frames(18);
  });

  frameScenario('sample_animated_positioned', (h) async {
    await h.mount((phase) => Stack(
          children: <Widget>[
            AnimatedPositioned(
              duration: const Duration(milliseconds: 400),
              curve: Curves.fastOutSlowIn,
              left: phase == 0 ? 0 : 300,
              top: 50,
              width: 100,
              height: 100,
              child: h.probe('box', const ColoredBox(color: orange)),
            ),
          ],
        ));
    await h.frames(2);
    h.setPhase(1);
    await h.frames(30);
  });

  frameScenario('sample_animated_rotation', (h) async {
    await h.mount((phase) => Center(
          child: AnimatedRotation(
            turns: phase == 0 ? 0 : 0.25,
            duration: const Duration(milliseconds: 300),
            child: h.probe(
              'box',
              const SizedBox(width: 100, height: 50, child: ColoredBox(color: blue)),
            ),
          ),
        ));
    await h.frames(2);
    h.setPhase(1);
    await h.frames(25);
  });

  frameScenario('sample_list_fling', (h) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    h.value('offset', () => controller.hasClients ? controller.offset : 0);
    await h.mount((phase) => ListView.builder(
          controller: controller,
          itemCount: 200,
          itemExtent: 50,
          itemBuilder: (context, index) {
            final row = ColoredBox(color: index.isEven ? blue : green);
            return index == 0 || index == 20 ? h.probe('row$index', row) : row;
          },
        ));
    await h.frames(1);
    await h.down(const Offset(400, 400));
    for (var step = 1; step <= 6; step++) {
      await h.frames(1);
      await h.move(Offset(400, 400 - 40.0 * step));
    }
    await h.frames(1);
    await h.up();
    await h.frames(90);
  });
}
