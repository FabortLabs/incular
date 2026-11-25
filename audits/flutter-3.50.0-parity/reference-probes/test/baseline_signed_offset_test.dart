import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Negative targets move real baselines above the parent', (
    WidgetTester tester,
  ) async {
    const text = ValueKey<String>('text');
    final baselineKey = GlobalKey();
    Widget build(double baseline, {bool hasBaseline = true}) => Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: Baseline(
          key: baselineKey,
          baseline: baseline,
          baselineType: TextBaseline.alphabetic,
          child: hasBaseline
              ? const Text('A', key: text, style: TextStyle(fontSize: 16))
              : const SizedBox(key: text, width: 100, height: 100),
        ),
      ),
    );

    await tester.pumpWidget(build(40));
    final outer = find.byKey(baselineKey);
    final child = find.byKey(text);
    final childSize = tester.getSize(child);
    final positiveTop =
        tester.getTopLeft(child).dy - tester.getTopLeft(outer).dy;
    expect(positiveTop, greaterThan(0));
    final render = tester.renderObject<RenderBaseline>(outer);
    expect(
      render.getDryBaseline(render.constraints, TextBaseline.alphabetic),
      40,
    );

    await tester.pumpWidget(build(-5));
    expect(tester.renderObject(outer), same(render));
    expect(tester.getSize(child), childSize);
    final top = tester.getTopLeft(child).dy - tester.getTopLeft(outer).dy;
    expect(top, positiveTop - 45);
    expect(top, lessThan(0));
    expect(
      tester.getSize(outer).height,
      (childSize.height + top).clamp(0, 600),
    );
    expect(
      render.getDryBaseline(render.constraints, TextBaseline.alphabetic),
      -5,
    );

    await tester.pumpWidget(build(-5, hasBaseline: false));
    expect(tester.renderObject(outer), same(render));
    expect(tester.getSize(outer), const Size(100, 0));
    expect(tester.getTopLeft(child).dy - tester.getTopLeft(outer).dy, -105);
    expect(
      render.getDryBaseline(render.constraints, TextBaseline.alphabetic),
      isNull,
    );
  });
}
