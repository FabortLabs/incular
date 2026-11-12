import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color teal = Color(0xFF009688);
const Color purple = Color(0xFF673AB7);

void main() {
  frameScenario('paint__opacity__phase_values', (h) async {
    await h.mount((phase) => Center(
          child: Opacity(
            opacity: phase == 0 ? 0.2 : 0.8,
            child: h.probe(
              'box',
              const SizedBox(
                width: 90,
                height: 60,
                child: ColoredBox(color: teal),
              ),
            ),
          ),
        ));
    await h.frames(2);
    h.setPhase(1);
    await h.frames(2);
  });

  frameScenario('paint__opacity__zero_keeps_layout', (h) async {
    await h.mount((_) => Center(
          child: Opacity(
            opacity: 0,
            child: h.probe(
              'box',
              const SizedBox(
                width: 80,
                height: 48,
                child: ColoredBox(color: purple),
              ),
            ),
          ),
        ));
    await h.frames(2);
  });

  frameScenario('paint__decorated_box__solid_background', (h) async {
    await h.mount((_) => Center(
          child: h.probe(
            'box',
            const DecoratedBox(
              decoration: BoxDecoration(color: teal),
              child: SizedBox(width: 104, height: 72),
            ),
          ),
        ));
    await h.frames(2);
  });

  frameScenario('paint__raw_image__intrinsic_scale', (h) async {
    final recorder = ui.PictureRecorder();
    final canvas = ui.Canvas(recorder);
    canvas.drawRect(
      const ui.Rect.fromLTWH(0, 0, 20, 10),
      ui.Paint()..color = teal,
    );
    final image = await recorder.endRecording().toImage(20, 10);
    addTearDown(image.dispose);
    await h.mount((_) => Center(
          child: h.probe(
            'image',
            RawImage(image: image, scale: 2),
          ),
        ));
    await h.frames(2);
  });
}

