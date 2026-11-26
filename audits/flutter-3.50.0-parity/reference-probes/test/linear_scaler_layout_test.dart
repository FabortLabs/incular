import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Clamped linear text uses the same measured size as the final factor',
    (tester) async {
      const key = ValueKey('label');
      Widget label(TextScaler scaler) => Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: Text('Scaled label', key: key, textScaler: scaler),
        ),
      );
      Future<Size> measure(TextScaler scaler) async {
        await tester.pumpWidget(label(scaler));
        return tester.getSize(find.byKey(key));
      }

      final clamped = const TextScaler.linear(5).clamp(maxScaleFactor: 3);
      final first = await measure(clamped);
      expect(first, await measure(const TextScaler.linear(3)));
      final reclamped = clamped.clamp(minScaleFactor: 4, maxScaleFactor: 6);
      final second = await measure(reclamped);
      expect(second, await measure(const TextScaler.linear(4)));
      expect(second.width, greaterThan(first.width));
      expect(second.height, greaterThan(first.height));
    },
  );
}
