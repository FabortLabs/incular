import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('IntrinsicHeight forwards the real baseline or its absence', (
    WidgetTester tester,
  ) async {
    const wrapper = ValueKey('wrapper');
    const leaf = ValueKey('leaf');
    const sibling = ValueKey('sibling');

    Widget frame({required bool suppress}) {
      const child = Text(
        'a',
        key: leaf,
        style: TextStyle(fontFamily: 'FlutterTest', fontSize: 128),
      );
      return Directionality(
        textDirection: TextDirection.ltr,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.baseline,
          textBaseline: TextBaseline.alphabetic,
          children: [
            IntrinsicHeight(
              key: wrapper,
              child: suppress ? const IgnoreBaseline(child: child) : child,
            ),
            const Text(
              'b',
              key: sibling,
              style: TextStyle(fontFamily: 'FlutterTest', fontSize: 32),
            ),
          ],
        ),
      );
    }

    await tester.pumpWidget(frame(suppress: false));
    final Size size = tester.getSize(find.byKey(wrapper));
    expect(tester.getTopLeft(find.byKey(leaf)).dy, 0);
    expect(tester.getTopLeft(find.byKey(sibling)).dy, 72);
    expect(tester.getSize(find.byKey(leaf)), size);

    await tester.pumpWidget(frame(suppress: true));
    expect(tester.getTopLeft(find.byKey(leaf)).dy, 0);
    expect(tester.getTopLeft(find.byKey(sibling)).dy, 0);
    expect(tester.getSize(find.byKey(wrapper)), size);

    await tester.pumpWidget(frame(suppress: false));
    expect(tester.getTopLeft(find.byKey(sibling)).dy, 72);
    expect(tester.getSize(find.byKey(wrapper)), size);
  });
}
