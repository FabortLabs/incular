import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a full-height column splits vertical wrap intrinsics', (
    tester,
  ) async {
    const probe = ValueKey<String>('intrinsic');
    const start = ValueKey<String>('start');
    const middle = ValueKey<String>('middle');
    const end = ValueKey<String>('end');
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: IntrinsicWidth(
            key: probe,
            child: Wrap(
              direction: Axis.vertical,
              children: [
                SizedBox(key: start, width: 80, height: 16),
                Column(
                  key: middle,
                  children: [SizedBox(width: 60, height: 40)],
                ),
                SizedBox(key: end, width: 48, height: 16),
              ],
            ),
          ),
        ),
      ),
    );
    final outer = find.byKey(probe);
    final origin = tester.getTopLeft(outer);
    expect(tester.getSize(outer), const Size(188, 600));
    for (final (key, size, x) in [
      (start, const Size(80, 16), 0.0),
      (middle, const Size(60, 600), 80.0),
      (end, const Size(48, 16), 140.0),
    ]) {
      final child = find.byKey(key);
      expect(tester.getSize(child), size);
      expect(tester.getTopLeft(child) - origin, Offset(x, 0));
    }
  });
}
