import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Wrap passes through the alphabetic text baseline', (
    tester,
  ) async {
    const probe = ValueKey('baseline');
    const glyph = Text(
      'X',
      textDirection: TextDirection.ltr,
      style: TextStyle(fontSize: 100, height: 1),
    );
    Future<Size> beside(Widget child) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: Baseline(
              key: probe,
              baseline: 175,
              baselineType: TextBaseline.alphabetic,
              child: child,
            ),
          ),
        ),
      );
      return tester.getSize(find.byKey(probe));
    }

    final direct = await beside(glyph);
    expect(direct.height, greaterThan(175));
    expect(await beside(const Wrap(children: [glyph])), direct);
  });

  testWidgets('Wrap uses the topmost actual baseline across reversed runs', (
    tester,
  ) async {
    const wrap = ValueKey('wrap');
    const first = ValueKey('first');
    const second = ValueKey('second');
    Widget frame(List<Widget> children) => Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 100,
          child: Wrap(
            key: wrap,
            verticalDirection: VerticalDirection.up,
            runSpacing: 7,
            children: children,
          ),
        ),
      ),
    );
    await tester.pumpWidget(
      frame(const [
        SizedBox(
          key: first,
          width: 80,
          height: 40,
          child: Text('X', style: TextStyle(fontSize: 30, height: 1)),
        ),
        SizedBox(
          key: second,
          width: 80,
          height: 20,
          child: Text('X', style: TextStyle(fontSize: 12, height: 1)),
        ),
      ]),
    );
    double? baseline(Key key) {
      final bool previous = RenderObject.debugCheckingIntrinsics;
      RenderObject.debugCheckingIntrinsics = true;
      try {
        return tester
            .renderObject<RenderBox>(find.byKey(key))
            .getDistanceToBaseline(TextBaseline.alphabetic, onlyReal: true);
      } finally {
        RenderObject.debugCheckingIntrinsics = previous;
      }
    }

    double offset(Key key) =>
        tester.getTopLeft(find.byKey(key)).dy -
        tester.getTopLeft(find.byKey(wrap)).dy;
    expect(offset(first), 27);
    final topmost = baseline(second)! + offset(second);
    expect(topmost, lessThan(baseline(first)! + offset(first)));
    expect(baseline(wrap), topmost);

    await tester.pumpWidget(frame(const [SizedBox(width: 80, height: 20)]));
    expect(baseline(wrap), isNull);
  });
}
