import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'IntrinsicWidth applies the stepped and clamped size to its child',
    (WidgetTester tester) async {
      const wrapper = ValueKey<String>('wrapper');
      const child = ValueKey<String>('child');

      for (final (width, height, limits, expected) in [
        (
          10.0,
          8.0,
          const BoxConstraints(maxWidth: 200, maxHeight: 200),
          const Size(110, 48),
        ),
        (
          7.0,
          null,
          const BoxConstraints(maxWidth: 200, maxHeight: 200),
          const Size(105, 47),
        ),
        (
          10.0,
          8.0,
          const BoxConstraints(maxWidth: 100, maxHeight: 46),
          const Size(100, 46),
        ),
        (
          10.0,
          8.0,
          const BoxConstraints.tightFor(width: 97, height: 43),
          const Size(97, 43),
        ),
      ]) {
        await tester.pumpWidget(
          Align(
            alignment: Alignment.topLeft,
            child: ConstrainedBox(
              constraints: limits,
              child: IntrinsicWidth(
                key: wrapper,
                stepWidth: width,
                stepHeight: height,
                child: const SizedBox(key: child, width: 103, height: 47),
              ),
            ),
          ),
        );
        expect(tester.getSize(find.byKey(wrapper)), expected);
        expect(tester.getSize(find.byKey(child)), expected);
        expect(
          tester.getTopLeft(find.byKey(wrapper)),
          tester.getTopLeft(find.byKey(child)),
        );
      }
    },
  );
}
