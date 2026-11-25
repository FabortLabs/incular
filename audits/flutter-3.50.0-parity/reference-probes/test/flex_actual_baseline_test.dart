import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget marker(double height, double baseline) => ConstrainedBox(
  constraints: BoxConstraints.tightFor(width: 100, height: height),
  child: Baseline(
    baseline: baseline,
    baselineType: TextBaseline.alphabetic,
    child: const Text('A', style: TextStyle(fontSize: 4)),
  ),
);

Widget outer(Widget nested, double baseline) => Directionality(
  textDirection: TextDirection.ltr,
  child: Row(
    crossAxisAlignment: CrossAxisAlignment.baseline,
    textBaseline: TextBaseline.alphabetic,
    children: [nested, marker(100, baseline)],
  ),
);

double nestedY(WidgetTester tester, Key key) =>
    tester.getTopLeft(find.byKey(key)).dy;

void main() {
  testWidgets('Nested Column exposes first logical child baseline', (
    WidgetTester tester,
  ) async {
    const key = ValueKey<String>('nested-column');
    const filler = SizedBox(width: 100, height: 100);
    Widget build(MainAxisAlignment main, VerticalDirection direction) => outer(
      Column(
        key: key,
        mainAxisAlignment: main,
        verticalDirection: direction,
        children: [filler, marker(100, 10), filler, marker(100, 10), filler],
      ),
      500,
    );
    RenderFlex? retained;
    for (final (main, direction, expected) in [
      (MainAxisAlignment.start, VerticalDirection.down, 110.0),
      (MainAxisAlignment.end, VerticalDirection.down, 210.0),
      (MainAxisAlignment.end, VerticalDirection.up, 310.0),
      (MainAxisAlignment.start, VerticalDirection.up, 410.0),
    ]) {
      await tester.pumpWidget(build(main, direction));
      final render = tester.renderObject<RenderFlex>(find.byKey(key));
      retained ??= render;
      expect(render, same(retained));
      expect(500 - nestedY(tester, key), expected);
    }
  });

  testWidgets('Nested Row exposes the highest placed baseline and clears it', (
    WidgetTester tester,
  ) async {
    const key = ValueKey<String>('nested-row');
    Widget build(bool real) => outer(
      Row(
        key: key,
        mainAxisSize: MainAxisSize.min,
        children: [
          real ? marker(100, 40) : const SizedBox(width: 100, height: 100),
          Flexible(
            child: real
                ? marker(60, 5)
                : const SizedBox(width: 100, height: 60),
          ),
        ],
      ),
      80,
    );

    await tester.pumpWidget(build(true));
    final render = tester.renderObject<RenderFlex>(find.byKey(key));
    expect(nestedY(tester, key), 55);
    await tester.pumpWidget(build(false));
    expect(tester.renderObject(find.byKey(key)), same(render));
    expect(nestedY(tester, key), 0);
  });
}
