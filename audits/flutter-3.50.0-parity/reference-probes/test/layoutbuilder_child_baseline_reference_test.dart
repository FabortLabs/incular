import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('LayoutBuilder forwards a laid-out child baseline', (
    tester,
  ) async {
    const leaf = ValueKey('text');
    const host = ValueKey('baseline');
    const text = Text(
      'X',
      key: leaf,
      textDirection: TextDirection.ltr,
      style: TextStyle(fontSize: 100, height: 1),
    );
    Future<double> top(Widget child) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: Baseline(
              key: host,
              baseline: 180,
              baselineType: TextBaseline.alphabetic,
              child: child,
            ),
          ),
        ),
      );
      return tester.getTopLeft(find.byKey(leaf)).dy -
          tester.getTopLeft(find.byKey(host)).dy;
    }

    final direct = await top(text);
    expect(direct, lessThan(180));
    expect(await top(LayoutBuilder(builder: (_, _) => text)), direct);

    const noBaseline = SizedBox(key: leaf, width: 60, height: 100);
    expect(await top(noBaseline), 80);
    expect(await top(LayoutBuilder(builder: (_, _) => noBaseline)), 80);
  });
}
