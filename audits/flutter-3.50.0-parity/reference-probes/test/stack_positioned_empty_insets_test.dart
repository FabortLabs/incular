import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('empty Positioned data participates in retained Stack sizing', (
    WidgetTester tester,
  ) async {
    const stackKey = ValueKey('stack');
    const childKey = ValueKey('child');
    Widget subject(double? left) => Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: Stack(
          key: stackKey,
          children: [
            Positioned(
              left: left,
              child: const SizedBox(key: childKey, width: 40, height: 20),
            ),
          ],
        ),
      ),
    );

    await tester.pumpWidget(subject(null));
    final Element retained = tester.element(find.byKey(childKey));
    expect(tester.getSize(find.byKey(stackKey)), const Size(40, 20));

    await tester.pumpWidget(subject(5));
    expect(tester.getSize(find.byKey(stackKey)), const Size(800, 600));
    expect(tester.getTopLeft(find.byKey(childKey)), const Offset(5, 0));
    expect(tester.element(find.byKey(childKey)), same(retained));

    await tester.pumpWidget(subject(null));
    expect(tester.getSize(find.byKey(stackKey)), const Size(40, 20));
    expect(tester.element(find.byKey(childKey)), same(retained));
  });
}
