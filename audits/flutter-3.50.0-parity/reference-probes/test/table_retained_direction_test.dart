import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('the retained table follows ambient direction until overridden', (
    tester,
  ) async {
    const firstKey = ValueKey('first');
    const secondKey = ValueKey('second');
    Table makeTable([TextDirection? explicit]) => Table(
      textDirection: explicit,
      columnWidths: const {0: FixedColumnWidth(56), 1: FixedColumnWidth(33)},
      children: const [
        TableRow(
          children: [
            SizedBox(key: firstKey, height: 20),
            SizedBox(key: secondKey, height: 20),
          ],
        ),
      ],
    );
    final original = makeTable();
    Widget host(TextDirection direction, Table table) => Directionality(
      textDirection: direction,
      child: Center(child: table),
    );
    List<double> lefts() {
      final x = tester.getTopLeft(find.byType(Table)).dx;
      return [
        tester.getTopLeft(find.byKey(firstKey)).dx - x,
        tester.getTopLeft(find.byKey(secondKey)).dx - x,
      ];
    }

    await tester.pumpWidget(host(TextDirection.ltr, original));
    final first = tester.renderObject(find.byKey(firstKey));
    expect(lefts(), [0, 56]);
    await tester.pumpWidget(host(TextDirection.rtl, original));
    expect(lefts(), [33, 0]);
    expect(tester.renderObject(find.byKey(firstKey)), same(first));
    await tester.pumpWidget(
      host(TextDirection.rtl, makeTable(TextDirection.ltr)),
    );
    expect(lefts(), [0, 56]);
    expect(tester.renderObject(find.byKey(firstKey)), same(first));
  });
}
