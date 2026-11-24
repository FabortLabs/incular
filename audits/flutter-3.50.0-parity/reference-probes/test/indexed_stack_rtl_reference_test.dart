import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final direction in TextDirection.values) {
    testWidgets('IndexedStack topStart for ${direction.name}', (tester) async {
      const stack = ValueKey<String>('stack');
      const selected = ValueKey<String>('selected');
      await tester.pumpWidget(
        Directionality(
          textDirection: direction,
          child: const Center(
            child: SizedBox(
              width: 100,
              height: 80,
              child: Align(
                alignment: Alignment.topLeft,
                child: IndexedStack(
                  key: stack,
                  index: 1,
                  children: <Widget>[
                    SizedBox(width: 80, height: 30),
                    SizedBox(key: selected, width: 20, height: 10),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      final stackFinder = find.byKey(stack);
      final selectedFinder = find.byKey(selected);
      expect(tester.getSize(stackFinder), const Size(80, 30));
      expect(tester.getSize(selectedFinder), const Size(20, 10));
      expect(
        tester.getTopLeft(selectedFinder) - tester.getTopLeft(stackFinder),
        direction == TextDirection.ltr ? Offset.zero : const Offset(60, 0),
      );
    });
  }
}
