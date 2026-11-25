import 'dart:ui' show SemanticsFlag;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('A cached semantic page is hidden until revealed', (
    WidgetTester tester,
  ) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: PageView(
          allowImplicitScrolling: true,
          children: const [Text('Page 0'), Text('Page 1'), Text('Page 2')],
        ),
      ),
    );
    Finder page(int i) => find.text('Page $i', skipOffstage: false);
    bool isHidden(int i) => tester
        .getSemantics(page(i))
        .getSemanticsData()
        .hasFlag(SemanticsFlag.isHidden);

    expect(isHidden(0), isFalse);
    expect(isHidden(1), isTrue);
    tester.renderObject(page(1)).showOnScreen();
    await tester.pumpAndSettle();
    expect(isHidden(0), isTrue);
    expect(isHidden(1), isFalse);
    handle.dispose();
  });
}
