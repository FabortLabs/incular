import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('default stack alignment follows the ambient start edge', (tester) async {
    const stack = ValueKey<String>('stack');
    const large = ValueKey<String>('large');
    const small = ValueKey<String>('small');
    const content = Center(
      child: Stack(
        key: stack,
        children: [
          SizedBox(key: large, width: 20, height: 20),
          SizedBox(key: small, width: 10, height: 10),
        ],
      ),
    );
    for (final direction in [TextDirection.ltr, TextDirection.rtl, TextDirection.ltr]) {
      await tester.pumpWidget(Directionality(textDirection: direction, child: content));
      final origin = tester.getTopLeft(find.byKey(stack));
      expect(tester.getSize(find.byKey(stack)), const Size(20, 20));
      expect(tester.getTopLeft(find.byKey(large)) - origin, Offset.zero);
      expect(tester.getTopLeft(find.byKey(small)) - origin,
        direction == TextDirection.ltr ? Offset.zero : const Offset(10, 0));
    }
  });

  testWidgets('only the default stack alignment mirrors in RTL', (tester) async {
    const stackKey = ValueKey('stackPx');
    const smallKey = ValueKey('smallPx');
    const children = [
      SizedBox(width: 20, height: 20),
      SizedBox(key: smallKey, width: 10, height: 10),
    ];
    for (final indexed in [false, true]) {
      for (final explicit in [false, true]) {
        final Widget content;
        if (indexed) {
          content = explicit
              ? const IndexedStack(key: stackKey, index: 1, alignment: Alignment.topLeft, children: children)
              : const IndexedStack(key: stackKey, index: 1, children: children);
        } else {
          content = explicit
              ? const Stack(key: stackKey, alignment: Alignment.topLeft, children: children)
              : const Stack(key: stackKey, children: children);
        }
        await tester.pumpWidget(Directionality(
          textDirection: TextDirection.rtl,
          child: Center(child: content),
        ));
        final origin = tester.getTopLeft(find.byKey(stackKey));
        expect(tester.getTopLeft(find.byKey(smallKey)) - origin,
          explicit ? Offset.zero : const Offset(10, 0));
      }
    }
  });
}
