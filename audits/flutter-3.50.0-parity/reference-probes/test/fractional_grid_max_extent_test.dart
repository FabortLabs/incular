import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('grid maximum cross-axis extent can be below one logical pixel', (
    tester,
  ) async {
    const gridKey = ValueKey('grid');
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 2,
            height: 1,
            child: GridView.extent(
              key: gridKey,
              maxCrossAxisExtent: 0.5,
              children: List.generate(4, (i) => SizedBox(key: ValueKey(i))),
            ),
          ),
        ),
      ),
    );
    final grid = tester.getTopLeft(find.byKey(gridKey));
    final fourth = find.byKey(const ValueKey(3));
    expect(tester.getSize(fourth), const Size(0.5, 0.5));
    expect(tester.getTopLeft(fourth) - grid, const Offset(1.5, 0));
  });
}
