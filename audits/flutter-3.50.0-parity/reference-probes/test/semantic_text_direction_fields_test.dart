import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Hints, tooltips and alternate values supply direction without a label',
    (tester) async {
      const keys = [
        ValueKey('hint'),
        ValueKey('tooltip'),
        ValueKey('increase'),
        ValueKey('decrease'),
      ];
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.rtl,
          child: Column(
            children: [
              Semantics(
                key: keys[0],
                hint: 'info',
                child: const SizedBox.square(dimension: 10),
              ),
              Semantics(
                key: keys[1],
                tooltip: 'preview',
                child: const SizedBox.square(dimension: 10),
              ),
              Semantics(
                key: keys[2],
                increasedValue: 'higher',
                child: const SizedBox.square(dimension: 10),
              ),
              Semantics(
                key: keys[3],
                decreasedValue: 'lower',
                child: const SizedBox.square(dimension: 10),
              ),
            ],
          ),
        ),
      );
      for (final key in keys) {
        final node = tester.getSemantics(find.byKey(key));
        expect(node.label, isEmpty);
        expect(node.textDirection, TextDirection.rtl);
      }
    },
  );
}
