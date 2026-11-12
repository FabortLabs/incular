// Implicit animation frame scenarios. Each one has a twin with the same name
// in harness/incular/tests/frames/implicit.rs.

import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color blue = Color(0xFF2196F3);
const Color orange = Color(0xFFFF5722);
const Color green = Color(0xFF4CAF50);

void main() {
  frameScenario('implicit__container__fields', (h) async {
    await h.mount(
      (phase) => Center(
        child: h.probe(
          'box',
          AnimatedContainer(
            duration: const Duration(milliseconds: 320),
            curve: Curves.easeInOut,
            width: phase == 0 ? 100 : 180,
            height: phase == 0 ? 70 : 110,
            margin: EdgeInsets.all(phase == 0 ? 8 : 18),
            padding: EdgeInsets.all(phase == 0 ? 4 : 12),
            alignment: phase == 0 ? Alignment.topLeft : Alignment.bottomRight,
            transform: Matrix4.translationValues(phase == 0 ? 3 : 23, 0, 0),
            transformAlignment: phase == 0
                ? Alignment.topLeft
                : Alignment.center,
            decoration: BoxDecoration(
              color: phase == 0 ? blue : orange,
              borderRadius: BorderRadius.circular(phase == 0 ? 2 : 18),
              border: Border.all(
                color: phase == 0 ? green : blue,
                width: phase == 0 ? 1 : 5,
              ),
            ),
            child: const ColoredBox(color: green),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__padding__edge_insets', (h) async {
    await h.mount(
      (phase) => Center(
        child: AnimatedPadding(
          padding: EdgeInsets.fromLTRB(
            phase == 0 ? 4 : 28,
            phase == 0 ? 8 : 20,
            phase == 0 ? 16 : 2,
            phase == 0 ? 3 : 23,
          ),
          duration: const Duration(milliseconds: 320),
          curve: Curves.easeInOut,
          child: h.probe(
            'box',
            const SizedBox(
              width: 80,
              height: 40,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__align__alignment_and_factors', (h) async {
    await h.mount(
      (phase) => Center(
        child: AnimatedAlign(
          alignment: phase == 0 ? Alignment.topLeft : Alignment.bottomRight,
          widthFactor: phase == 0 ? 1 : 2,
          heightFactor: phase == 0 ? 1 : 1.5,
          duration: const Duration(milliseconds: 320),
          curve: Curves.easeInOut,
          child: h.probe(
            'box',
            const SizedBox(
              width: 64,
              height: 32,
              child: ColoredBox(color: orange),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__positioned__edges_and_size', (h) async {
    await h.mount(
      (phase) => Stack(
        children: <Widget>[
          AnimatedPositioned(
            duration: const Duration(milliseconds: 320),
            curve: Curves.easeInOut,
            left: phase == 0 ? 10 : 310,
            top: phase == 0 ? 20 : 140,
            width: phase == 0 ? 70 : 120,
            height: phase == 0 ? 50 : 90,
            child: h.probe('box', const ColoredBox(color: green)),
          ),
        ],
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__positioned_directional__direction', (h) async {
    await h.mount(
      (phase) => Directionality(
        textDirection: phase == 0 ? TextDirection.ltr : TextDirection.rtl,
        child: Stack(
          children: <Widget>[
            AnimatedPositionedDirectional(
              duration: const Duration(milliseconds: 320),
              curve: Curves.easeInOut,
              start: phase == 0 ? 12 : 44,
              top: 24,
              width: 80,
              height: 40,
              child: h.probe('box', const ColoredBox(color: blue)),
            ),
          ],
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__scale__alignment', (h) async {
    await h.mount(
      (phase) => Center(
        child: AnimatedScale(
          scale: phase == 0 ? 1 : 1.7,
          alignment: Alignment.bottomLeft,
          duration: const Duration(milliseconds: 320),
          curve: Curves.easeInOut,
          child: h.probe(
            'box',
            const SizedBox(
              width: 100,
              height: 60,
              child: ColoredBox(color: green),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__rotation__turns_and_alignment', (h) async {
    await h.mount(
      (phase) => Center(
        child: AnimatedRotation(
          turns: phase == 0 ? 0 : 0.375,
          alignment: Alignment.topRight,
          duration: const Duration(milliseconds: 320),
          curve: Curves.easeInOut,
          child: h.probe(
            'box',
            const SizedBox(
              width: 100,
              height: 60,
              child: ColoredBox(color: orange),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__slide__fractional_offset', (h) async {
    await h.mount(
      (phase) => Center(
        child: AnimatedSlide(
          offset: phase == 0 ? Offset.zero : const Offset(0.75, -0.5),
          duration: const Duration(milliseconds: 320),
          curve: Curves.easeInOut,
          child: h.probe(
            'box',
            const SizedBox(
              width: 80,
              height: 40,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__opacity__opacity', (h) async {
    await h.mount(
      (phase) => Center(
        child: AnimatedOpacity(
          opacity: phase == 0 ? 1 : 0.25,
          duration: const Duration(milliseconds: 320),
          curve: Curves.easeInOut,
          child: h.probe(
            'box',
            const SizedBox(
              width: 80,
              height: 40,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__sliver_opacity__opacity', (h) async {
    await h.mount(
      (phase) => CustomScrollView(
        slivers: <Widget>[
          SliverAnimatedOpacity(
            opacity: phase == 0 ? 1 : 0.25,
            duration: const Duration(milliseconds: 320),
            curve: Curves.easeInOut,
            sliver: SliverToBoxAdapter(
              child: h.probe(
                'box',
                const SizedBox(
                  width: 80,
                  height: 40,
                  child: ColoredBox(color: green),
                ),
              ),
            ),
          ),
        ],
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__default_text_style__text_style', (h) async {
    await h.mount(
      (phase) => Center(
        child: AnimatedDefaultTextStyle(
          style: TextStyle(
            fontFamily: 'Inter',
            fontSize: phase == 0 ? 14 : 30,
            color: phase == 0 ? const Color(0xFF000000) : blue,
          ),
          duration: const Duration(milliseconds: 320),
          curve: Curves.easeInOut,
          child: h.probe('text', const Text('Inter 123')),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__physical_model__color_elevation_radius', (h) async {
    await h.mount(
      (phase) => Center(
        child: h.probe(
          'model',
          AnimatedPhysicalModel(
            color: phase == 0 ? blue : orange,
            shadowColor: phase == 0 ? green : blue,
            elevation: phase == 0 ? 1 : 18,
            borderRadius: BorderRadius.circular(phase == 0 ? 2 : 24),
            duration: const Duration(milliseconds: 320),
            curve: Curves.easeInOut,
            clipBehavior: Clip.antiAlias,
            child: const SizedBox(width: 100, height: 60),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__fractional_sized_box__factors', (h) async {
    await h.mount(
      (phase) => Center(
        child: SizedBox(
          width: 240,
          height: 180,
          child: AnimatedFractionallySizedBox(
            alignment: phase == 0 ? Alignment.topLeft : Alignment.bottomRight,
            widthFactor: phase == 0 ? 0.25 : 0.75,
            heightFactor: phase == 0 ? 0.4 : 0.8,
            duration: const Duration(milliseconds: 320),
            curve: Curves.easeInOut,
            child: h.probe('box', const ColoredBox(color: orange)),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__tween_animation_builder__initial_and_retarget', (
    h,
  ) async {
    var sampled = 0.0;
    var ended = 0;
    h.value('tween', () => sampled);
    h.value('on_end', () => ended.toDouble());
    await h.mount(
      (phase) => TweenAnimationBuilder<double>(
        tween: Tween<double>(
          begin: phase == 0 ? 0 : 500,
          end: phase == 0 ? 100 : 240,
        ),
        duration: const Duration(milliseconds: 320),
        curve: Curves.easeInOut,
        onEnd: () => ended++,
        builder: (context, value, child) {
          sampled = value;
          return SizedBox(width: 80, height: 40, child: child);
        },
        child: h.probe('box', const ColoredBox(color: blue)),
      ),
    );
    await h.frames(8);
    h.setPhase(1);
    await h.frames(24);
  });

  frameScenario('implicit__retarget__current_value_and_on_end', (h) async {
    var ended = 0;
    h.value('on_end', () => ended.toDouble());
    await h.mount(
      (phase) => Center(
        child: AnimatedOpacity(
          opacity: phase == 0 ? 1 : (phase == 1 ? 0 : 0.55),
          duration: const Duration(milliseconds: 320),
          curve: Curves.easeInOut,
          onEnd: () => ended++,
          child: h.probe(
            'box',
            const SizedBox(
              width: 80,
              height: 40,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(7);
    h.setPhase(2);
    await h.frames(24);
  });

  frameScenario('implicit__zero_duration__completion', (h) async {
    var ended = 0;
    h.value('on_end', () => ended.toDouble());
    await h.mount(
      (phase) => Center(
        child: AnimatedOpacity(
          opacity: phase == 0 ? 1 : 0.3,
          duration: Duration.zero,
          onEnd: () => ended++,
          child: h.probe(
            'box',
            const SizedBox(
              width: 80,
              height: 40,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(4);
  });

  frameScenario('implicit__disabled_animations__reduced_motion', (h) async {
    final dispatcher = h.tester.binding.platformDispatcher;
    addTearDown(dispatcher.clearAccessibilityFeaturesTestValue);
    await h.mount(
      (phase) => Center(
        child: AnimatedOpacity(
          opacity: phase == 0 ? 1 : 0.3,
          duration: const Duration(milliseconds: 400),
          curve: Curves.linear,
          child: h.probe(
            'box',
            const SizedBox(
              width: 80,
              height: 40,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
    await h.frames(2);
    dispatcher.accessibilityFeaturesTestValue = const FakeAccessibilityFeatures(
      disableAnimations: true,
    );
    h.setPhase(1);
    await h.frames(8);
  });
}
