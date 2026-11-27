import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a placeholder child keeps its own size and pointer target', (
    tester,
  ) async {
    var taps = 0;
    await tester.pumpWidget(
      Center(
        child: Placeholder(
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () => taps++,
            child: const SizedBox(width: 20, height: 30),
          ),
        ),
      ),
    );
    expect(tester.getSize(find.byType(Placeholder)), const Size(20, 30));
    await tester.tap(find.byType(Placeholder));
    expect(taps, 1);
  });

  testWidgets('the placeholder outline does not absorb a hit', (tester) async {
    var taps = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Stack(
          children: [
            Positioned.fill(
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => taps++,
                child: const SizedBox.expand(),
              ),
            ),
            const Positioned.fill(child: Placeholder()),
          ],
        ),
      ),
    );
    await tester.tapAt(const Offset(10, 10));
    expect(taps, 1);
  });
}
