import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color blue = Color(0xFF2196F3);
const Color orange = Color(0xFFFF5722);
const Color green = Color(0xFF4CAF50);

void main() {
  frameScenario('transitions__compositor__fade_slide_scale_rotation', (h) async {
    final controller = AnimationController(vsync: h.tester, duration: const Duration(milliseconds: 200), value: 0);
    addTearDown(controller.dispose);
    h.value('value', () => controller.value);
    await h.mount((_) => Center(
          child: FadeTransition(
            opacity: controller,
            child: SlideTransition(
              position: Tween<Offset>(begin: Offset.zero, end: const Offset(0.4, -0.2)).animate(controller),
              textDirection: TextDirection.ltr,
              child: ScaleTransition(
                scale: controller,
                alignment: Alignment.center,
                child: RotationTransition(
                  turns: controller,
                  child: h.probe('box', const SizedBox(width: 80, height: 40, child: ColoredBox(color: blue))),
                ),
              ),
            ),
          ),
        ));
    await h.frames(2);
    controller.value = 0.6;
    await h.frames(1);
    controller.value = 1;
    await h.frames(2);
  });

  frameScenario('transitions__layout__align', (h) async {
    final controller = AnimationController(vsync: h.tester, duration: const Duration(milliseconds: 200), value: 0.25);
    addTearDown(controller.dispose);
    h.value('value', () => controller.value);
    await h.mount((_) => Center(
          child: SizedBox(
            width: 240,
            height: 120,
            child: AlignTransition(
              alignment: AlignmentTween(begin: Alignment.topLeft, end: Alignment.bottomRight).animate(controller),
              child: h.probe('box', const SizedBox(width: 100, height: 50, child: ColoredBox(color: orange))),
            ),
          ),
        ));
    await h.frames(2);
    controller.value = 0.5;
    await h.frames(1);
    controller.value = 1;
    await h.frames(2);
  });

  frameScenario('transitions__layout__size_clip', (h) async {
    final controller = AnimationController(vsync: h.tester, duration: const Duration(milliseconds: 200), value: 1);
    addTearDown(controller.dispose);
    h.value('value', () => controller.value);
    await h.mount((_) => Center(
          child: SizedBox(
            width: 240,
            height: 120,
            child: h.probe(
              'box',
              SizeTransition(
                axis: Axis.horizontal,
                axisAlignment: -1,
                fixedCrossAxisSizeFactor: 1,
                sizeFactor: controller,
                child: const SizedBox(width: 100, height: 50, child: ColoredBox(color: orange)),
              ),
            ),
          ),
        ));
    await h.frames(2);
    controller.value = 0.5;
    await h.frames(1);
    controller.value = 1;
    await h.frames(2);
  });

  frameScenario('transitions__positioned__relative_rect', (h) async {
    final controller = AnimationController(vsync: h.tester, duration: const Duration(milliseconds: 200), value: 0);
    addTearDown(controller.dispose);
    h.value('value', () => controller.value);
    await h.mount((_) => Center(
          child: SizedBox(
            width: 300,
            height: 180,
            child: Stack(children: [
              PositionedTransition(
                rect: RelativeRectTween(begin: const RelativeRect.fromLTRB(0, 0, 200, 130), end: const RelativeRect.fromLTRB(120, 60, 20, 10)).animate(controller),
                child: h.probe('positioned', const ColoredBox(color: green)),
              ),
              RelativePositionedTransition(
                rect: RectTween(begin: const Rect.fromLTWH(10, 20, 60, 50), end: const Rect.fromLTWH(180, 100, 100, 70)).animate(controller),
                size: const Size(300, 180),
                child: h.probe('relative', const ColoredBox(color: blue)),
              ),
            ]),
          ),
        ));
    await h.frames(2);
    controller.value = 0.5;
    await h.frames(1);
    controller.value = 1;
    await h.frames(2);
  });

  frameScenario('transitions__builders__cached_child', (h) async {
    final controller = AnimationController(vsync: h.tester, duration: const Duration(milliseconds: 200), value: 0);
    addTearDown(controller.dispose);
    var animatedBuilds = 0;
    var listenableBuilds = 0;
    h.value('value', () => controller.value);
    h.value('animated_builds', () => animatedBuilds.toDouble());
    h.value('listenable_builds', () => listenableBuilds.toDouble());
    await h.mount((_) => Center(child: Column(mainAxisSize: MainAxisSize.min, children: [
          AnimatedBuilder(
            animation: controller,
            child: h.probe('animated_child', const SizedBox(width: 30, height: 20, child: ColoredBox(color: blue))),
            builder: (context, child) {
              animatedBuilds++;
              return Transform.translate(offset: Offset(controller.value * 40, 0), child: child);
            },
          ),
          ListenableBuilder(
            listenable: controller,
            child: h.probe('listenable_child', const SizedBox(width: 30, height: 20, child: ColoredBox(color: orange))),
            builder: (context, child) {
              listenableBuilds++;
              return Transform.translate(offset: Offset(0, controller.value * 30), child: child);
            },
          ),
        ])));
    await h.frames(1);
    controller.value = 0.5;
    await h.frames(1);
    controller.value = 1;
    await h.frames(1);
  });

  frameScenario('transitions__paint__decoration_matrix', (h) async {
    final controller = AnimationController(vsync: h.tester, duration: const Duration(milliseconds: 200), value: 0);
    addTearDown(controller.dispose);
    h.value('value', () => controller.value);
    await h.mount((_) => Center(child: Column(mainAxisSize: MainAxisSize.min, children: [
          h.probe(
            'decoration',
            DecoratedBoxTransition(
              decoration: DecorationTween(
                begin: const BoxDecoration(color: blue),
                end: const BoxDecoration(color: orange),
              ).animate(controller),
              child: const SizedBox(width: 80, height: 40),
            ),
          ),
          MatrixTransition(
            animation: controller,
            onTransform: (value) => Matrix4.translationValues(value * 60, 0, 0),
            child: h.probe('matrix', const SizedBox(width: 50, height: 30, child: ColoredBox(color: green))),
          ),
        ])));
    await h.frames(2);
    controller.value = 0.5;
    await h.frames(1);
    controller.value = 1;
    await h.frames(2);
  });

  frameScenario('transitions__text__default_style', (h) async {
    final controller = AnimationController(vsync: h.tester, duration: const Duration(milliseconds: 200), value: 0);
    addTearDown(controller.dispose);
    h.value('value', () => controller.value);
    await h.mount((_) => Center(
          child: DefaultTextStyleTransition(
            style: TextStyleTween(
              begin: const TextStyle(fontFamily: 'Inter', fontSize: 14, color: Color(0xFF000000)),
              end: const TextStyle(fontFamily: 'Inter', fontSize: 22, color: blue),
            ).animate(controller),
            textAlign: TextAlign.center,
            softWrap: false,
            child: h.probe('text', const Text('Explicit transition text')),
          ),
        ));
    await h.frames(2);
    controller.value = 0.5;
    await h.frames(1);
    controller.value = 1;
    await h.frames(2);
  });

  frameScenario('transitions__sliver_fade__opacity', (h) async {
    final controller = AnimationController(vsync: h.tester, duration: const Duration(milliseconds: 200), value: 0.25);
    addTearDown(controller.dispose);
    h.value('opacity', () => controller.value);
    await h.mount((_) => CustomScrollView(slivers: [
          SliverFadeTransition(
            opacity: controller,
            sliver: SliverToBoxAdapter(
              child: h.probe('sliver_child', const SizedBox(height: 80, child: ColoredBox(color: green))),
            ),
          ),
        ]));
    await h.frames(1);
    controller.value = 0.75;
    await h.frames(1);
  });
}

