import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget looseFrame(Widget child, double width, double height) => Directionality(
  textDirection: TextDirection.ltr,
  child: Center(
    child: SizedBox(
      width: width,
      height: height,
      child: Align(alignment: Alignment.topLeft, child: child),
    ),
  ),
);
void main() {
  testWidgets('default OverflowBox fills a loose bounded parent', (t) async {
    const p = ValueKey('p'), c = ValueKey('c');
    await t.pumpWidget(
      looseFrame(
        const OverflowBox(
          key: p,
          child: SizedBox(key: c, width: 20, height: 10),
        ),
        100,
        80,
      ),
    );
    expect(t.getSize(find.byKey(p)), const Size(100, 80));
    expect(
      t.getTopLeft(find.byKey(c)) - t.getTopLeft(find.byKey(p)),
      const Offset(40, 35),
    );
  });
  testWidgets('loose Contain FittedBox preserves aspect ratio', (t) async {
    const p = ValueKey('p');
    await t.pumpWidget(
      looseFrame(
        const FittedBox(key: p, child: SizedBox(width: 200, height: 100)),
        100,
        100,
      ),
    );
    expect(t.getSize(find.byKey(p)), const Size(100, 50));
  });
  testWidgets('baseline can place child above parent', (t) async {
    const p = ValueKey('p'), c = ValueKey('c');
    await t.pumpWidget(
      looseFrame(
        const Baseline(
          key: p,
          baseline: 5,
          baselineType: TextBaseline.alphabetic,
          child: SizedBox(key: c, width: 20, height: 20),
        ),
        100,
        80,
      ),
    );
    expect(t.getSize(find.byKey(p)), const Size(20, 5));
    expect(
      t.getTopLeft(find.byKey(c)) - t.getTopLeft(find.byKey(p)),
      const Offset(0, -15),
    );
  });
}
