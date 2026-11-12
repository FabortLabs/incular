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
}
