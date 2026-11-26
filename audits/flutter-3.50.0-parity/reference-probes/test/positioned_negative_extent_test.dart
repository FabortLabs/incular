import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('negative positioned extent tightens to zero, not natural size', (
    tester,
  ) async {
    const key = ValueKey('child');
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: Stack(
          children: [
            Positioned(
              width: -5,
              child: SizedBox(key: key, width: 25, height: 10),
            ),
          ],
        ),
      ),
    );
    expect(tester.getSize(find.byKey(key)), const Size(0, 10));
  });
}
