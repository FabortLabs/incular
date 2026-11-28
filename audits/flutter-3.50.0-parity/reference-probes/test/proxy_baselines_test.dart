import 'dart:ui' as ui show ImageFilter;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _MeasureBaseline extends SingleChildRenderObjectWidget {
  const _MeasureBaseline({super.key, required super.child});

  @override
  _RenderMeasureBaseline createRenderObject(BuildContext context) =>
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
  testWidgets('semantics and backdrop proxies forward and clear real baselines', (
    tester,
  ) async {
    const bounds = BoxConstraints.tightFor(width: 40, height: 20);
    const outer = ValueKey('outer');
    const leaf = ValueKey('leaf');
    const proxy = ValueKey('proxy');
    for (final isBackdrop in [false, true]) {
      RenderBox? retained;
      for (final hasBaseline in [false, true, null, false]) {
        final Widget? child = hasBaseline == null
            ? null
            : _MeasureBaseline(
                key: leaf,
                child: hasBaseline
                    ? const Text('Ag', textDirection: TextDirection.ltr)
                    : const SizedBox(width: 40, height: 20),
              );
        await tester.pumpWidget(
          Center(
            child: SizedBox(
              width: 40,
              height: 20,
              child: _MeasureBaseline(
                key: outer,
                child: isBackdrop
                    ? BackdropFilter(
                        key: proxy,
                        filter: ui.ImageFilter.blur(sigmaX: 1, sigmaY: 1),
                        child: child,
                      )
                    : Semantics(key: proxy, child: child),
              ),
            ),
          ),
        );
        final render = tester.renderObject<RenderBox>(find.byKey(proxy));
        retained ??= render;
        expect(render, same(retained));
        expect(render.getDryLayout(bounds), const Size(40, 20));

        final measured = tester
            .renderObject<_RenderMeasureBaseline>(find.byKey(outer))
            .actual;
        final measuredLeaf = hasBaseline == null
            ? null
            : tester.renderObject<_RenderMeasureBaseline>(find.byKey(leaf));
        final dry = measuredLeaf?.getDryBaseline(bounds, TextBaseline.alphabetic);
        expect(measured != null, hasBaseline == true);
        expect(measured, measuredLeaf?.actual);
        expect(dry != null, hasBaseline == true);
        expect(render.getDryBaseline(bounds, TextBaseline.alphabetic), dry);
      }
    }
  });
}
