import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _outer = ValueKey('outer');
const _stack = ValueKey('stack');
const _leaf = ValueKey('text');
const _children = [
  _MeasureBaseline(
    key: _leaf,
    child: Text('X', style: TextStyle(fontSize: 100, height: 1)),
  ),
  SizedBox(width: 200, height: 200),
];

class _MeasureBaseline extends SingleChildRenderObjectWidget {
  const _MeasureBaseline({super.key, required super.child});

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderMeasureBaseline();
}

class _RenderMeasureBaseline extends RenderProxyBox {
  double? actual;

  @override
  void performLayout() {
    super.performLayout();
    actual = child?.getDistanceToBaseline(
      TextBaseline.alphabetic,
      onlyReal: true,
    );
  }
}

void main() {
  testWidgets(
    'retained IndexedStack forwards only the selected actual baseline',
    (tester) async {
      for (final index in <int?>[0, null, 1, 0]) {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Center(
              child: Baseline(
                key: _outer,
                baseline: 300,
                baselineType: TextBaseline.alphabetic,
                child: _MeasureBaseline(
                  key: _stack,
                  child: IndexedStack(
                    index: index,
                    alignment: Alignment.center,
                    children: _children,
                  ),
                ),
              ),
            ),
          ),
        );
        final stack = tester.renderObject<_RenderMeasureBaseline>(
          find.byKey(_stack),
        );
        final leaf = tester.renderObject<_RenderMeasureBaseline>(
          find.byKey(_leaf, skipOffstage: false),
        );
        final leafBaseline = leaf.actual!;
        final leafOffset = (200 - leaf.size.height) / 2;
        final expected = index == 0 ? leafOffset + leafBaseline : null;
        expect(stack.size, const Size(200, 200));
        expect(stack.actual, expected);
        expect(
          tester.getSize(find.byKey(_outer)).height,
          500 - (expected ?? 200),
        );
      }
    },
  );

  testWidgets('Stack reports the minimum positioned actual baseline', (
    tester,
  ) async {
    const upper = ValueKey('upper');
    const lower = ValueKey('lower');
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: _MeasureBaseline(
            key: _stack,
            child: Stack(
              children: [
                SizedBox(width: 200, height: 200),
                Positioned(
                  top: 29,
                  child: _MeasureBaseline(key: lower, child: Text('X')),
                ),
                Positioned(
                  top: 3,
                  child: _MeasureBaseline(key: upper, child: Text('X')),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    final stack = tester.renderObject<_RenderMeasureBaseline>(
      find.byKey(_stack),
    );
    final a = tester.renderObject<_RenderMeasureBaseline>(find.byKey(upper));
    final b = tester.renderObject<_RenderMeasureBaseline>(find.byKey(lower));
    final higher = a.actual! + 3;
    final farther = b.actual! + 29;
    expect(farther, greaterThan(higher));
    expect(stack.actual, higher);
  });
}
