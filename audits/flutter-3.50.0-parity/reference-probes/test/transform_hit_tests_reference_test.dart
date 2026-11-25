import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a retained transform can keep hits at its layout position', (
    tester,
  ) async {
    const marker = ValueKey('marker');
    final positions = <Offset>[];
    Element? first;
    for (final transformHits in [true, false]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Stack(
            children: [
              Positioned(
                left: 60,
                top: 70,
                child: Transform(
                  transform: Matrix4.identity()
                    ..translateByDouble(50, 0, 0, 1)
                    ..scaleByDouble(2, 2, 1, 1),
                  origin: const Offset(3, 4),
                  alignment: Alignment.center,
                  transformHitTests: transformHits,
                  child: GestureDetector(
                    key: marker,
                    onTapDown: (event) => positions.add(event.localPosition),
                    onTap: () {},
                    child: const SizedBox(
                      width: 40,
                      height: 30,
                      child: ColoredBox(color: Color(0xffffffff)),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      first ??= tester.element(find.byKey(marker));
      expect(tester.element(find.byKey(marker)), same(first));
      expect(
        tester.getRect(find.byKey(marker)),
        const Rect.fromLTWH(87, 51, 80, 60),
      );
      await tester.tapAt(const Offset(65, 74));
      await tester.tapAt(const Offset(120, 78));
      expect(positions, [
        transformHits ? const Offset(16.5, 13.5) : const Offset(5, 4),
      ]);
      positions.clear();
    }
  });
}
