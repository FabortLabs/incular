import 'package:flutter/widgets.dart';

import 'recorder.dart';

const Color blue = Color(0xFF2196F3);
const Color orange = Color(0xFFFF5722);
const Color green = Color(0xFF4CAF50);

class _SingleDelegate extends SingleChildLayoutDelegate {
  _SingleDelegate();

  @override
  Size getSize(BoxConstraints constraints) =>
      constraints.constrain(const Size(300, 200));

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) =>
      const BoxConstraints.tightFor(width: 70, height: 40);

  @override
  Offset getPositionForChild(Size size, Size childSize) => const Offset(40, 30);

  @override
  bool shouldRelayout(covariant _SingleDelegate oldDelegate) => false;
}

class _MultiDelegate extends MultiChildLayoutDelegate {
  _MultiDelegate();

  @override
  Size getSize(BoxConstraints constraints) =>
      constraints.constrain(const Size(200, 150));

  @override
  void performLayout(Size size) {
    layoutChild('a', const BoxConstraints.tightFor(width: 40, height: 30));
    positionChild('a', const Offset(20, 25));
    layoutChild('b', const BoxConstraints.tightFor(width: 50, height: 20));
    positionChild('b', const Offset(90, 70));
  }

  @override
  bool shouldRelayout(covariant _MultiDelegate oldDelegate) => false;
}

class _FlowDelegate extends FlowDelegate {
  _FlowDelegate();

  @override
  Size getSize(BoxConstraints constraints) =>
      constraints.constrain(const Size(300, 200));

  @override
  void paintChildren(FlowPaintingContext context) {
    context.paintChild(0, transform: Matrix4.translationValues(40, 50, 0));
    context.paintChild(1, transform: Matrix4.translationValues(120, 80, 0));
  }

  @override
  bool shouldRepaint(covariant _FlowDelegate oldDelegate) => false;
}

void main() {
  frameScenario('layout__row__main_and_cross_alignment', (h) async {
    await h.mount(
      (_) => Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: <Widget>[
          h.probe(
            'row0',
            const SizedBox(
              width: 50,
              height: 20,
              child: ColoredBox(color: blue),
            ),
          ),
          h.probe(
            'row1',
            const SizedBox(
              width: 80,
              height: 40,
              child: ColoredBox(color: orange),
            ),
          ),
          h.probe(
            'row2',
            const SizedBox(
              width: 60,
              height: 30,
              child: ColoredBox(color: green),
            ),
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__flex__expanded_and_spacing', (h) async {
    await h.mount(
      (_) => Row(
        children: <Widget>[
          Expanded(child: h.probe('expanded0', const ColoredBox(color: blue))),
          const SizedBox(width: 12),
          Expanded(
            flex: 2,
            child: h.probe('expanded1', const ColoredBox(color: orange)),
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__stack__positioned', (h) async {
    await h.mount(
      (_) => Stack(
        children: <Widget>[
          h.probe(
            'stack_base',
            const SizedBox(
              width: 300,
              height: 200,
              child: ColoredBox(color: blue),
            ),
          ),
          Positioned(
            left: 35,
            top: 45,
            width: 70,
            height: 50,
            child: h.probe('positioned', const ColoredBox(color: orange)),
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__indexed_stack__active_child', (h) async {
    await h.mount(
      (_) => IndexedStack(
        index: 1,
        children: <Widget>[
          h.probe(
            'indexed0',
            const SizedBox(
              width: 100,
              height: 40,
              child: ColoredBox(color: blue),
            ),
          ),
          h.probe(
            'indexed1',
            const SizedBox(
              width: 140,
              height: 60,
              child: ColoredBox(color: orange),
            ),
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__wrap__multiple_runs', (h) async {
    await h.mount(
      (_) => SizedBox(
        width: 230,
        child: Wrap(
          spacing: 10,
          runSpacing: 8,
          alignment: WrapAlignment.spaceBetween,
          children: <Widget>[
            h.probe(
              'wrap0',
              const SizedBox(
                width: 70,
                height: 20,
                child: ColoredBox(color: blue),
              ),
            ),
            h.probe(
              'wrap1',
              const SizedBox(
                width: 80,
                height: 30,
                child: ColoredBox(color: orange),
              ),
            ),
            h.probe(
              'wrap2',
              const SizedBox(
                width: 90,
                height: 25,
                child: ColoredBox(color: green),
              ),
            ),
            h.probe(
              'wrap3',
              const SizedBox(
                width: 60,
                height: 35,
                child: ColoredBox(color: blue),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__aspect_constraints_and_fraction', (h) async {
    await h.mount(
      (_) => Center(
        child: SizedBox(
          width: 300,
          height: 180,
          child: Align(
            alignment: Alignment.topLeft,
            child: h.probe(
              'aspect',
              AspectRatio(
                aspectRatio: 2,
                child: ConstrainedBox(
                  constraints: const BoxConstraints(
                    maxWidth: 200,
                    maxHeight: 120,
                  ),
                  child: FractionallySizedBox(
                    widthFactor: 0.5,
                    heightFactor: 0.5,
                    child: const ColoredBox(color: green),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__fitted_box__contain_alignment', (h) async {
    await h.mount(
      (_) => Center(
        child: h.probe(
          'fitted',
          const SizedBox(
            width: 160,
            height: 100,
            child: FittedBox(
              fit: BoxFit.contain,
              alignment: Alignment.bottomRight,
              child: SizedBox(
                width: 100,
                height: 80,
                child: ColoredBox(color: orange),
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__padding_and_overflow_boxes', (h) async {
    await h.mount(
      (_) => Center(
        child: h.probe(
          'padding',
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 15),
            child: SizedOverflowBox(
              size: const Size(120, 70),
              alignment: Alignment.bottomRight,
              child: const SizedBox(
                width: 180,
                height: 90,
                child: ColoredBox(color: green),
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__table__rows_and_columns', (h) async {
    await h.mount(
      (_) => Center(
        child: Table(
          defaultColumnWidth: const IntrinsicColumnWidth(),
          children: <TableRow>[
            TableRow(
              children: <Widget>[
                h.probe(
                  'table0',
                  const SizedBox(height: 30, child: ColoredBox(color: blue)),
                ),
                const SizedBox(
                  width: 80,
                  height: 40,
                  child: ColoredBox(color: orange),
                ),
              ],
            ),
            TableRow(
              children: <Widget>[
                const SizedBox(height: 50, child: ColoredBox(color: green)),
                const SizedBox(
                  width: 100,
                  height: 35,
                  child: ColoredBox(color: blue),
                ),
              ],
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__custom_single_child', (h) async {
    await h.mount(
      (_) => Center(
        child: h.probe(
          'custom_single',
          CustomSingleChildLayout(
            delegate: _SingleDelegate(),
            child: const SizedBox(
              width: 70,
              height: 40,
              child: ColoredBox(color: blue),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__custom_multi_child', (h) async {
    await h.mount(
      (_) => Center(
        child: CustomMultiChildLayout(
          delegate: _MultiDelegate(),
          children: <Widget>[
            LayoutId(
              id: 'a',
              child: h.probe('multi_a', const ColoredBox(color: blue)),
            ),
            LayoutId(
              id: 'b',
              child: h.probe('multi_b', const ColoredBox(color: orange)),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__flow__child_transforms', (h) async {
    await h.mount(
      (_) => Center(
        child: Flow(
          delegate: _FlowDelegate(),
          children: <Widget>[
            h.probe(
              'flow0',
              const SizedBox(
                width: 40,
                height: 30,
                child: ColoredBox(color: blue),
              ),
            ),
            h.probe(
              'flow1',
              const SizedBox(
                width: 50,
                height: 40,
                child: ColoredBox(color: orange),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__visibility_offstage_and_transform', (h) async {
    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            h.probe(
              'transformed',
              Transform.translate(
                offset: const Offset(25, 15),
                child: const SizedBox(
                  width: 60,
                  height: 40,
                  child: ColoredBox(color: blue),
                ),
              ),
            ),
            Offstage(
              offstage: true,
              child: h.probe(
                'offstage',
                const SizedBox(
                  width: 90,
                  height: 25,
                  child: ColoredBox(color: orange),
                ),
              ),
            ),
            Visibility(
              visible: false,
              maintainState: true,
              maintainAnimation: true,
              child: h.probe(
                'hidden',
                const SizedBox(
                  width: 50,
                  height: 20,
                  child: ColoredBox(color: green),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__baseline__cross_axis', (h) async {
    await h.mount(
      (_) => Row(
        crossAxisAlignment: CrossAxisAlignment.baseline,
        textBaseline: TextBaseline.alphabetic,
        children: <Widget>[
          h.probe(
            'baseline0',
            const Text('Inter 12', style: TextStyle(fontSize: 12)),
          ),
          h.probe(
            'baseline1',
            const Text('Inter 24', style: TextStyle(fontSize: 24)),
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__layout_builder__constraint_size', (h) async {
    await h.mount(
      (_) => Center(
        child: LayoutBuilder(
          builder: (context, constraints) {
            return SizedBox(
              width: constraints.maxWidth / 2,
              height: 40,
              child: h.probe('layout_builder', const ColoredBox(color: blue)),
            );
          },
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__intrinsic_width__step', (h) async {
    await h.mount(
      (_) => Center(
        child: h.probe(
          'intrinsic',
          IntrinsicWidth(
            stepWidth: 20,
            child: const SizedBox(
              width: 83,
              height: 40,
              child: ColoredBox(color: orange),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__box_constraints__limited_overflow_intrinsic', (
    h,
  ) async {
    await h.mount(
      (_) => Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          h.probe(
            'limited',
            UnconstrainedBox(
              child: LimitedBox(
                maxWidth: 120,
                maxHeight: 60,
                child: SizedBox.expand(child: const ColoredBox(color: blue)),
              ),
            ),
          ),
          SizedBox(
            width: 100,
            height: 80,
            child: OverflowBox(
              minWidth: 150,
              maxWidth: 150,
              minHeight: 110,
              maxHeight: 110,
              child: h.probe('overflow', const ColoredBox(color: orange)),
            ),
          ),
          h.probe(
            'intrinsic_height',
            IntrinsicHeight(
              child: Row(
                children: <Widget>[
                  const SizedBox(
                    width: 40,
                    height: 20,
                    child: ColoredBox(color: green),
                  ),
                  const SizedBox(
                    width: 40,
                    height: 50,
                    child: ColoredBox(color: blue),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__flex__main_alignment_variants', (h) async {
    Widget alignedRow(MainAxisAlignment alignment, String tag) {
      return SizedBox(
        width: 300,
        height: 30,
        child: Row(
          mainAxisAlignment: alignment,
          children: <Widget>[
            h.probe(
              '${tag}0',
              const SizedBox(width: 40, height: 20, child: ColoredBox(color: blue)),
            ),
            h.probe(
              '${tag}1',
              const SizedBox(width: 60, height: 20, child: ColoredBox(color: orange)),
            ),
          ],
        ),
      );
    }

    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            alignedRow(MainAxisAlignment.start, 'start'),
            alignedRow(MainAxisAlignment.end, 'end'),
            alignedRow(MainAxisAlignment.center, 'center'),
            alignedRow(MainAxisAlignment.spaceBetween, 'between'),
            alignedRow(MainAxisAlignment.spaceAround, 'around'),
            alignedRow(MainAxisAlignment.spaceEvenly, 'evenly'),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                h.probe(
                  'min0',
                  const SizedBox(width: 40, height: 20, child: ColoredBox(color: blue)),
                ),
                h.probe(
                  'min1',
                  const SizedBox(width: 60, height: 20, child: ColoredBox(color: orange)),
                ),
              ],
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__flex__direction_reversal', (h) async {
    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            SizedBox(
              width: 300,
              height: 20,
              child: Row(
                textDirection: TextDirection.rtl,
                children: <Widget>[
                  h.probe(
                    'rtl0',
                    const SizedBox(width: 50, height: 20, child: ColoredBox(color: blue)),
                  ),
                  const Spacer(),
                  h.probe(
                    'rtl1',
                    const SizedBox(width: 80, height: 20, child: ColoredBox(color: orange)),
                  ),
                ],
              ),
            ),
            SizedBox(
              width: 300,
              height: 60,
              child: Row(
                verticalDirection: VerticalDirection.up,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  h.probe(
                    'vup0',
                    const SizedBox(width: 50, height: 20, child: ColoredBox(color: green)),
                  ),
                  h.probe(
                    'vup1',
                    const SizedBox(width: 40, height: 40, child: ColoredBox(color: blue)),
                  ),
                ],
              ),
            ),
            SizedBox(
              width: 80,
              height: 120,
              child: Column(
                verticalDirection: VerticalDirection.up,
                mainAxisAlignment: MainAxisAlignment.start,
                children: <Widget>[
                  h.probe(
                    'cup0',
                    const SizedBox(width: 30, height: 20, child: ColoredBox(color: orange)),
                  ),
                  h.probe(
                    'cup1',
                    const SizedBox(width: 30, height: 40, child: ColoredBox(color: green)),
                  ),
                ],
              ),
            ),
            SizedBox(
              width: 100,
              height: 60,
              child: Column(
                textDirection: TextDirection.rtl,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  h.probe(
                    'ctr0',
                    const SizedBox(width: 30, height: 20, child: ColoredBox(color: blue)),
                  ),
                  h.probe(
                    'ctr1',
                    const SizedBox(width: 60, height: 20, child: ColoredBox(color: orange)),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__flex__fit_loose_and_tight', (h) async {
    await h.mount(
      (_) => Center(
        child: SizedBox(
          width: 400,
          height: 40,
          child: Row(
            children: <Widget>[
              Flexible(
                child: h.probe('fit_loose_color', const ColoredBox(color: blue)),
              ),
              Flexible(
                child: h.probe(
                  'fit_loose_sized',
                  const SizedBox(width: 30, height: 20, child: ColoredBox(color: orange)),
                ),
              ),
              Expanded(
                child: h.probe(
                  'fit_tight_sized',
                  const SizedBox(width: 30, height: 20, child: ColoredBox(color: green)),
                ),
              ),
              Flexible(
                child: h.probe(
                  'fit_loose_fraction',
                  FractionallySizedBox(
                    widthFactor: 1,
                    child: const ColoredBox(color: blue),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__flex__spacing_and_fixed_gaps', (h) async {
    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            SizedBox(
              width: 250,
              height: 30,
              child: Row(
                spacing: 10,
                children: <Widget>[
                  h.probe(
                    'sp0',
                    const SizedBox(width: 40, height: 20, child: ColoredBox(color: blue)),
                  ),
                  h.probe(
                    'sp1',
                    const SizedBox(width: 60, height: 20, child: ColoredBox(color: orange)),
                  ),
                  h.probe(
                    'sp2',
                    const SizedBox(width: 30, height: 20, child: ColoredBox(color: green)),
                  ),
                ],
              ),
            ),
            SizedBox(
              width: 200,
              height: 30,
              child: Row(
                spacing: 10,
                mainAxisAlignment: MainAxisAlignment.center,
                children: <Widget>[
                  h.probe(
                    'flex_fixed',
                    const SizedBox(width: 60, height: 20, child: ColoredBox(color: blue)),
                  ),
                  Expanded(
                    child: h.probe('flex_fill', const ColoredBox(color: orange)),
                  ),
                ],
              ),
            ),
            Row(
              mainAxisSize: MainAxisSize.min,
              spacing: 8,
              children: <Widget>[
                h.probe(
                  'minrow0',
                  const SizedBox(width: 20, height: 20, child: ColoredBox(color: green)),
                ),
                h.probe(
                  'minrow1',
                  const SizedBox(width: 30, height: 20, child: ColoredBox(color: blue)),
                ),
              ],
            ),
            Column(
              mainAxisSize: MainAxisSize.min,
              spacing: 6,
              children: <Widget>[
                h.probe(
                  'vcol0',
                  const SizedBox(width: 40, height: 10, child: ColoredBox(color: blue)),
                ),
                h.probe(
                  'vcol1',
                  const SizedBox(width: 40, height: 10, child: ColoredBox(color: orange)),
                ),
                h.probe(
                  'vcol2',
                  const SizedBox(width: 40, height: 10, child: ColoredBox(color: green)),
                ),
              ],
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__stack__fit_alignment_positioned', (h) async {
    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            SizedBox(
              width: 300,
              height: 100,
              child: Stack(
                alignment: Alignment.bottomRight,
                children: <Widget>[
                  h.probe(
                    's_loose',
                    const SizedBox(width: 60, height: 40, child: ColoredBox(color: blue)),
                  ),
                ],
              ),
            ),
            SizedBox(
              width: 300,
              height: 100,
              child: Stack(
                fit: StackFit.expand,
                children: <Widget>[
                  h.probe('s_expand', const ColoredBox(color: orange)),
                ],
              ),
            ),
            SizedBox(
              width: 300,
              height: 100,
              child: Align(
                alignment: Alignment.topLeft,
                child: Stack(
                  children: <Widget>[
                    h.probe(
                      's_intrinsic',
                      const SizedBox(width: 70, height: 30, child: ColoredBox(color: green)),
                    ),
                  ],
                ),
              ),
            ),
            SizedBox(
              width: 300,
              height: 100,
              child: Stack(
                children: <Widget>[
                  const SizedBox(
                    width: 300,
                    height: 100,
                    child: ColoredBox(color: blue),
                  ),
                  Positioned(
                    right: 10,
                    bottom: 5,
                    width: 30,
                    height: 20,
                    child: h.probe('s_pos', const ColoredBox(color: orange)),
                  ),
                ],
              ),
            ),
            SizedBox(
              width: 300,
              height: 100,
              child: Stack(
                children: <Widget>[
                  const SizedBox(
                    width: 300,
                    height: 100,
                    child: ColoredBox(color: blue),
                  ),
                  Positioned(
                    left: -20,
                    right: 10,
                    top: 10,
                    height: 30,
                    child: h.probe('s_neg', const ColoredBox(color: orange)),
                  ),
                ],
              ),
            ),
            SizedBox(
              width: 300,
              height: 100,
              child: Center(
                child: Stack(
                  fit: StackFit.passthrough,
                  children: <Widget>[
                    h.probe(
                      's_pass',
                      const SizedBox(width: 900, height: 50, child: ColoredBox(color: green)),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__stack__empty_and_positioned_only', (h) async {
    await h.mount(
      (_) => Center(
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            SizedBox(
              width: 150,
              height: 100,
              child: h.probe('empty_stack', Stack(children: const <Widget>[])),
            ),
            SizedBox(
              width: 150,
              height: 100,
              child: h.probe(
                'positioned_only',
                Stack(
                  children: <Widget>[
                    Positioned(
                      left: 10,
                      top: 10,
                      width: 50,
                      height: 20,
                      child: const ColoredBox(color: blue),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__wrap__alignment_and_run_axes', (h) async {
    await h.mount(
      (_) => Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          SizedBox(
            width: 230,
            height: 120,
            child: Wrap(
              spacing: 10,
              runSpacing: 8,
              alignment: WrapAlignment.center,
              runAlignment: WrapAlignment.end,
              crossAxisAlignment: WrapCrossAlignment.end,
              children: <Widget>[
                h.probe(
                  'wa0',
                  const SizedBox(width: 70, height: 20, child: ColoredBox(color: blue)),
                ),
                h.probe(
                  'wa1',
                  const SizedBox(width: 80, height: 30, child: ColoredBox(color: orange)),
                ),
                h.probe(
                  'wa2',
                  const SizedBox(width: 90, height: 25, child: ColoredBox(color: green)),
                ),
                h.probe(
                  'wa3',
                  const SizedBox(width: 60, height: 35, child: ColoredBox(color: blue)),
                ),
              ],
            ),
          ),
          SizedBox(
            width: 200,
            height: 150,
            child: Wrap(
              direction: Axis.vertical,
              spacing: 6,
              runSpacing: 10,
              alignment: WrapAlignment.start,
              runAlignment: WrapAlignment.spaceBetween,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: <Widget>[
                h.probe(
                  'vw0',
                  const SizedBox(width: 40, height: 30, child: ColoredBox(color: blue)),
                ),
                h.probe(
                  'vw1',
                  const SizedBox(width: 60, height: 50, child: ColoredBox(color: orange)),
                ),
                h.probe(
                  'vw2',
                  const SizedBox(width: 30, height: 40, child: ColoredBox(color: green)),
                ),
                h.probe(
                  'vw3',
                  const SizedBox(width: 50, height: 20, child: ColoredBox(color: blue)),
                ),
              ],
            ),
          ),
          SizedBox(
            width: 230,
            child: Wrap(
              textDirection: TextDirection.rtl,
              spacing: 10,
              runSpacing: 8,
              children: <Widget>[
                h.probe(
                  'rw0',
                  const SizedBox(width: 70, height: 20, child: ColoredBox(color: blue)),
                ),
                h.probe(
                  'rw1',
                  const SizedBox(width: 80, height: 30, child: ColoredBox(color: orange)),
                ),
                h.probe(
                  'rw2',
                  const SizedBox(width: 90, height: 25, child: ColoredBox(color: green)),
                ),
              ],
            ),
          ),
          SizedBox(
            width: 230,
            child: Wrap(
              alignment: WrapAlignment.spaceAround,
              children: <Widget>[
                h.probe(
                  'sa0',
                  const SizedBox(width: 70, height: 20, child: ColoredBox(color: blue)),
                ),
                h.probe(
                  'sa1',
                  const SizedBox(width: 80, height: 30, child: ColoredBox(color: orange)),
                ),
              ],
            ),
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__fitted_box__all_fits', (h) async {
    Widget fitted(BoxFit fit, String tag, [AlignmentGeometry alignment = Alignment.center]) {
      return SizedBox(
        width: 120,
        height: 80,
        child: FittedBox(
          fit: fit,
          alignment: alignment,
          child: h.probe(
            tag,
            const SizedBox(width: 200, height: 100, child: ColoredBox(color: blue)),
          ),
        ),
      );
    }

    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Row(
              mainAxisSize: MainAxisSize.min,
              spacing: 10,
              children: <Widget>[
                fitted(BoxFit.fill, 'fb_fill'),
                fitted(BoxFit.contain, 'fb_contain'),
                fitted(BoxFit.cover, 'fb_cover'),
                fitted(BoxFit.fitWidth, 'fb_fit_width'),
              ],
            ),
            const SizedBox(height: 10),
            Row(
              mainAxisSize: MainAxisSize.min,
              spacing: 10,
              children: <Widget>[
                fitted(BoxFit.fitHeight, 'fb_fit_height'),
                fitted(BoxFit.none, 'fb_none'),
                fitted(BoxFit.scaleDown, 'fb_scale_down'),
              ],
            ),
            const SizedBox(height: 10),
            Row(
              mainAxisSize: MainAxisSize.min,
              spacing: 10,
              children: <Widget>[
                fitted(BoxFit.contain, 'fb_contain_tl', Alignment.topLeft),
                fitted(BoxFit.cover, 'fb_cover_br', Alignment.bottomRight),
                fitted(BoxFit.none, 'fb_none_tl', Alignment.topLeft),
              ],
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__aspect_ratio__constraint_cases', (h) async {
    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            SizedBox(
              width: 800,
              height: 100,
              child: Center(
                child: h.probe(
                  'ar_wide',
                  AspectRatio(
                    aspectRatio: 4,
                    child: const ColoredBox(color: blue),
                  ),
                ),
              ),
            ),
            SizedBox(
              width: 300,
              height: 100,
              child: h.probe(
                'ar_tight',
                AspectRatio(
                  aspectRatio: 2,
                  child: const ColoredBox(color: orange),
                ),
              ),
            ),
            SizedBox(
              width: 800,
              height: 100,
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 200, maxHeight: 300),
                  child: h.probe(
                    'ar_narrow',
                    AspectRatio(
                      aspectRatio: 0.5,
                      child: const ColoredBox(color: green),
                    ),
                  ),
                ),
              ),
            ),
            SizedBox(
              width: 800,
              height: 100,
              child: Center(
                child: h.probe(
                  'ar_child',
                  AspectRatio(
                    aspectRatio: 2,
                    child: h.probe(
                      'ar_child_inner',
                      const SizedBox(width: 40, height: 40, child: ColoredBox(color: blue)),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__container__composition_order', (h) async {
    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            h.probe(
              'container',
              Container(
                width: 160,
                height: 100,
                margin: const EdgeInsets.all(10),
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
                color: blue,
                alignment: Alignment.bottomRight,
                child: h.probe(
                  'container_child',
                  const SizedBox(width: 30, height: 20, child: ColoredBox(color: orange)),
                ),
              ),
            ),
            const SizedBox(height: 20),
            h.probe(
              'rotated',
              Container(
                width: 100,
                height: 50,
                color: orange,
                transform: Matrix4.rotationZ(0.5),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__intrinsic_width__steps_and_flex_child', (h) async {
    await h.mount(
      (_) => Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          h.probe(
            'iw_steps',
            IntrinsicWidth(
              stepWidth: 25,
              stepHeight: 15,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  const SizedBox(width: 60, height: 22, child: ColoredBox(color: blue)),
                  const SizedBox(width: 80, height: 10, child: ColoredBox(color: orange)),
                ],
              ),
            ),
          ),
          h.probe(
            'iw_flex',
            IntrinsicWidth(
              child: Row(
                children: <Widget>[
                  const SizedBox(width: 40, height: 20, child: ColoredBox(color: green)),
                  Expanded(
                    child: h.probe(
                      'iw_flex_child',
                      const SizedBox(width: 20, height: 20, child: ColoredBox(color: blue)),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__baseline__without_baseline_child', (h) async {
    await h.mount(
      (_) => Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: <Widget>[
              h.probe(
                'bl_box',
                const SizedBox(width: 40, height: 30, child: ColoredBox(color: blue)),
              ),
              h.probe(
                'bl_text',
                const Text('Inter 24', style: TextStyle(fontSize: 24)),
              ),
            ],
          ),
          Column(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: <Widget>[
              h.probe(
                'col_text',
                const Text('Inter 12', style: TextStyle(fontSize: 12)),
              ),
              h.probe(
                'col_box',
                const SizedBox(width: 40, height: 30, child: ColoredBox(color: orange)),
              ),
            ],
          ),
        ],
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__unconstrained_box__constrained_axis', (h) async {
    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            SizedBox(
              width: 300,
              height: 200,
              child: UnconstrainedBox(
                clipBehavior: Clip.hardEdge,
                constrainedAxis: Axis.horizontal,
                child: h.probe(
                  'ub_h',
                  const SizedBox(width: 400, height: 260, child: ColoredBox(color: blue)),
                ),
              ),
            ),
            SizedBox(
              width: 300,
              height: 200,
              child: UnconstrainedBox(
                clipBehavior: Clip.hardEdge,
                constrainedAxis: Axis.vertical,
                child: h.probe(
                  'ub_v',
                  const SizedBox(width: 400, height: 100, child: ColoredBox(color: orange)),
                ),
              ),
            ),
            SizedBox(
              width: 300,
              height: 200,
              child: UnconstrainedBox(
                clipBehavior: Clip.hardEdge,
                child: h.probe(
                  'ub_none',
                  const SizedBox(width: 400, height: 260, child: ColoredBox(color: green)),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__layout_builder__rebuild_triggers', (h) async {
    var builds = 0;
    h.value('builds', () => builds.toDouble());
    await h.mount(
      (phase) => Center(
        child: SizedBox(
          width: phase == 0 ? 300 : 200,
          child: LayoutBuilder(
            builder: (context, constraints) {
              builds++;
              return h.probe(
                'lb_rebuild',
                const SizedBox(height: 40, child: ColoredBox(color: blue)),
              );
            },
          ),
        ),
      ),
    );
    await h.frames(2);
    h.setPhase(1);
    await h.frames(2);
    h.setPhase(2);
    await h.frames(2);
  });

  frameScenario('layout__visibility__maintain_flags', (h) async {
    await h.mount(
      (_) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            h.probe(
              'vis_size',
              Visibility(
                visible: false,
                maintainSize: true,
                maintainAnimation: true,
                maintainState: true,
                child: const SizedBox(width: 50, height: 20, child: ColoredBox(color: blue)),
              ),
            ),
            h.probe(
              'vis_state',
              Visibility(
                visible: false,
                maintainState: true,
                child: const SizedBox(width: 60, height: 10, child: ColoredBox(color: orange)),
              ),
            ),
            h.probe(
              'vis_default',
              const Visibility(
                visible: false,
                child: SizedBox(width: 40, height: 30, child: ColoredBox(color: green)),
              ),
            ),
            h.probe(
              'vis_replacement',
              Visibility(
                visible: false,
                replacement: const SizedBox(width: 25, height: 25, child: ColoredBox(color: blue)),
                child: const SizedBox(width: 40, height: 30, child: ColoredBox(color: orange)),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('layout__box_constraints__helper_values', (h) async {
    const c = BoxConstraints(
      minWidth: 50,
      maxWidth: 200,
      minHeight: 20,
      maxHeight: 100,
    );
    h.value('constrain_w', () => c.constrain(const Size(10, 500)).width);
    h.value('constrain_h', () => c.constrain(const Size(10, 500)).height);
    h.value('loosen_min_w', () => c.loosen().minWidth);
    h.value('tighten_min_w', () => c.tighten(width: 120).minWidth);
    h.value('tighten_max_w', () => c.tighten(width: 120).maxWidth);
    h.value('deflate_min_w', () => c.deflate(const EdgeInsets.all(10)).minWidth);
    h.value('deflate_max_w', () => c.deflate(const EdgeInsets.all(10)).maxWidth);
    h.value('biggest_w', () => c.biggest.width);
    h.value('smallest_h', () => c.smallest.height);
    h.value('tight_min_w', () => BoxConstraints.tight(const Size(70, 40)).minWidth);
    h.value('bounded_w', () => c.hasBoundedWidth ? 1 : 0);
    h.value('unbounded_w', () => const BoxConstraints(minWidth: 1).hasBoundedWidth ? 1 : 0);
    h.value('enforce_min_w', () => c.enforce(const BoxConstraints(minWidth: 100, maxWidth: 150)).minWidth);
    await h.mount((_) => const SizedBox(width: 10, height: 10, child: ColoredBox(color: blue)));
    await h.frames(1);
  });

  frameScenario('layout__table__default_flex_columns', (h) async {
    await h.mount(
      (_) => Center(
        child: h.probe(
          'table',
          Table(
            children: <TableRow>[
              TableRow(
                children: <Widget>[
                  const SizedBox(height: 30, child: ColoredBox(color: blue)),
                  const SizedBox(width: 80, height: 40, child: ColoredBox(color: orange)),
                ],
              ),
              TableRow(
                children: <Widget>[
                  const SizedBox(height: 50, child: ColoredBox(color: green)),
                  const SizedBox(width: 100, height: 35, child: ColoredBox(color: blue)),
                ],
              ),
            ],
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  // Orchestrator: a null factor keeps the parent minimum in Flutter
  // (shifted_box.dart _getInnerConstraints) under a tight parent.
  frameScenario('layout__fractionally_sized_box__null_factor_tight_parent', (h) async {
    await h.mount((_) => Center(
          child: SizedBox(
            width: 200,
            height: 100,
            child: FractionallySizedBox(
              heightFactor: 0.5,
              child: h.probe('child', const SizedBox(width: 50, height: 20, child: ColoredBox(color: blue))),
            ),
          ),
        ));
    await h.frames(1);
  });
}
