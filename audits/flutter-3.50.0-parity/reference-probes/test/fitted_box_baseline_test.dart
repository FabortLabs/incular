import 'package:flutter/rendering.dart' show RenderBox, RenderProxyBox;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _BaselineProbe extends SingleChildRenderObjectWidget {
  const _BaselineProbe({required this.observe, required super.child});

  final ValueChanged<double?> observe;

  @override
  _RenderBaselineProbe createRenderObject(BuildContext context) =>
      _RenderBaselineProbe(observe);

  @override
  void updateRenderObject(BuildContext context, _RenderBaselineProbe render) {
    render
      ..observe = observe
      ..markNeedsLayout();
  }
}

class _RenderBaselineProbe extends RenderProxyBox {
  _RenderBaselineProbe(this.observe);

  ValueChanged<double?> observe;

  @override
  void performLayout() {
    super.performLayout();
    observe(
      child!.getDistanceToBaseline(TextBaseline.alphabetic, onlyReal: true),
    );
  }
}

void main() {
  testWidgets('FittedBox forwards the real untransformed child baseline', (
    tester,
  ) async {
    const fitKey = ValueKey('fit');
    const leafKey = ValueKey('leaf');
    RenderBox? retained;
    for (final hasBaseline in [true, false, true]) {
      final fittedValues = <double?>[];
      final leafValues = <double?>[];
      final Widget child = hasBaseline
          ? const Text(
              'Ag',
              key: leafKey,
              textDirection: TextDirection.ltr,
              style: TextStyle(fontSize: 16),
            )
          : const SizedBox(key: leafKey, width: 30, height: 20);
      await tester.pumpWidget(
        Center(
          child: SizedBox.square(
            dimension: 200,
            child: _BaselineProbe(
              observe: fittedValues.add,
              child: FittedBox(
                key: fitKey,
                child: _BaselineProbe(observe: leafValues.add, child: child),
              ),
            ),
          ),
        ),
      );
      final fit = tester.renderObject<RenderBox>(find.byKey(fitKey));
      final leaf = tester.renderObject<RenderBox>(find.byKey(leafKey));
      retained ??= fit;
      expect(fit, same(retained));
      expect(leaf.getTransformTo(fit).storage[0], greaterThan(1));
      final actual = leafValues.last;
      expect(actual != null, hasBaseline);
      expect(fittedValues.last, actual);
    }
  });
}
