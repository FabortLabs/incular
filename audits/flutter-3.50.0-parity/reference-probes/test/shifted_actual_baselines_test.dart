import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const childKey = ValueKey<String>('child');
const wrapperKey = ValueKey<String>('wrapper');

Widget marker(bool real) => SizedBox(
  key: childKey,
  width: 100,
  height: 100,
  child: real
      ? const Baseline(
          baseline: 40,
          baselineType: TextBaseline.alphabetic,
          child: Text('A', style: TextStyle(fontSize: 4)),
        )
      : null,
);

Widget host(Widget wrapper, {double minHeight = 0}) => Directionality(
  textDirection: TextDirection.ltr,
  child: Align(
    alignment: Alignment.topLeft,
    child: ConstrainedBox(
      constraints: BoxConstraints(
        maxWidth: 200,
        minHeight: minHeight,
        maxHeight: 200,
      ),
      child: wrapper,
    ),
  ),
);

(Size, Offset, double?) read(WidgetTester tester) {
  final wrapper = find.byKey(wrapperKey);
  final child = find.byKey(childKey);
  final render = tester.renderObject<RenderBox>(wrapper);
  return (
    tester.getSize(wrapper),
    tester.getTopLeft(child) - tester.getTopLeft(wrapper),
    render.getDryBaseline(render.constraints, TextBaseline.alphabetic),
  );
}

void main() {
  testWidgets('Align carries the placed baseline and clears it after update', (
    WidgetTester tester,
  ) async {
    RenderBox? retained;
    for (final (align, xy, metric, real) in [
      (Alignment.topLeft, 0.0, 40.0, true),
      (Alignment.center, 50.0, 90.0, true),
      (Alignment.bottomRight, 100.0, 140.0, true),
      (Alignment.bottomRight, 100.0, null, false),
    ]) {
      await tester.pumpWidget(
        host(Align(key: wrapperKey, alignment: align, child: marker(real))),
      );
      final render = tester.renderObject<RenderBox>(find.byKey(wrapperKey));
      retained ??= render;
      expect(render, same(retained));
      expect(read(tester), (const Size(200, 200), Offset(xy, xy), metric));
    }
  });

  testWidgets('Padding adds its top inset to a real baseline', (
    WidgetTester tester,
  ) async {
    for (final real in [true, false]) {
      await tester.pumpWidget(
        host(
          Padding(
            key: wrapperKey,
            padding: const EdgeInsets.fromLTRB(7, 13, 15, 11),
            child: marker(real),
          ),
        ),
      );
      expect(read(tester), (
        const Size(122, 124),
        const Offset(7, 13),
        real ? 53.0 : null,
      ));
    }
  });

  testWidgets('Fractional box applies constrained height to child baseline', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      host(
        FractionallySizedBox(
          key: wrapperKey,
          widthFactor: 0.5,
          heightFactor: 0.5,
          child: marker(true),
        ),
        minHeight: 160,
      ),
    );
    expect(read(tester), (const Size(100, 160), const Offset(0, 30), 70.0));
  });
}
