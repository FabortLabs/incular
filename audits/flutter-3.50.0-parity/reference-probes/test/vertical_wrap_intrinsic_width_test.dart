import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a full-height column splits vertical wrap intrinsics', (
    tester,
  ) async {
    const probe = ValueKey<String>('intrinsic');
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: IntrinsicWidth(
            key: probe,
            child: Wrap(
              direction: Axis.vertical,
              children: [
                SizedBox(width: 80, height: 16),
                Column(children: [SizedBox(width: 60, height: 40)]),
                SizedBox(width: 48, height: 16),
              ],
            ),
          ),
        ),
      ),
    );
    expect(tester.getSize(find.byKey(probe)).width, 188);
  });
}
