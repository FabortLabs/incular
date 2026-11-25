import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'scoped scaling controls paragraph geometry, not explicit scale',
    (tester) async {
      const inherited = ValueKey('inherited');
      const explicit = ValueKey('explicit');
      const child = Directionality(
        textDirection: TextDirection.ltr,
        child: Column(
          children: [
            Text('Preview', key: inherited, style: TextStyle(fontSize: 14)),
            Text(
              'Preview',
              key: explicit,
              style: TextStyle(fontSize: 14),
              textScaler: TextScaler.noScaling,
            ),
          ],
        ),
      );
      Future<void> pump(double factor) => tester.pumpWidget(
        MediaQuery(
          data: MediaQueryData(textScaler: TextScaler.linear(factor)),
          child: child,
        ),
      );

      await pump(1);
      final identity = tester.element(find.byKey(inherited));
      final before = tester.getSize(find.byKey(inherited));
      expect(tester.getSize(find.byKey(explicit)), before);

      await pump(2);
      final scaled = tester.getSize(find.byKey(inherited));
      expect(scaled.width, greaterThan(before.width * 1.9));
      expect(scaled.height, greaterThan(before.height * 1.9));
      expect(tester.element(find.byKey(inherited)), same(identity));
      expect(tester.getSize(find.byKey(explicit)), before);

      await pump(0);
      expect(tester.getSize(find.byKey(inherited)).width, 0);
      expect(tester.element(find.byKey(inherited)), same(identity));
      expect(tester.getSize(find.byKey(explicit)), before);
    },
  );
}
