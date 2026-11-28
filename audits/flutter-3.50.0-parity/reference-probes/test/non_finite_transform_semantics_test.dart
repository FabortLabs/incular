import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('pinned Flutter diagnoses a non-finite semantics rectangle', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    try {
      for (final value in [1.0, double.nan]) {
        await tester.pumpWidget(
          Transform(
            transform: Matrix4.identity()..storage[0] = value,
            child: ClipRRect(
              borderRadius: const BorderRadius.all(Radius.circular(4)),
              child: Semantics(
                container: true,
                label: 'transformed label',
                textDirection: TextDirection.ltr,
                child: const SizedBox.expand(),
              ),
            ),
          ),
        );
        final error = tester.takeException();
        if (value.isFinite) {
          expect(error, isNull);
          expect(find.bySemanticsLabel('transformed label'), findsOneWidget);
        } else {
          expect(error, isA<AssertionError>());
          expect(error.toString(), contains('non-finite rect'));
        }
      }
    } finally {
      handle.dispose();
    }
  });
}
