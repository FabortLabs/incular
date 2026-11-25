import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Fill only takes the row height while IntrinsicHeight sets it', (
    tester,
  ) async {
    const first = ValueKey('first');
    const second = ValueKey('second');
    const third = ValueKey('third');
    Widget host(Table table) => Directionality(
      textDirection: TextDirection.rtl,
      child: Center(child: SizedBox(width: 300, child: table)),
    );
    Table table(TableCellVerticalAlignment alignment) => Table(
      defaultVerticalAlignment: TableCellVerticalAlignment.middle,
      children: [
        TableRow(
          children: [
            TableCell(
              verticalAlignment: alignment,
              child: const SizedBox(key: first, height: 100),
            ),
            const TableCell(
              verticalAlignment: TableCellVerticalAlignment.top,
              child: SizedBox(key: second, height: 40),
            ),
            const TableCell(
              verticalAlignment: TableCellVerticalAlignment.bottom,
              child: SizedBox(key: third, height: 10),
            ),
          ],
        ),
      ],
    );
    Rect local(Key key) {
      final origin = tester.getTopLeft(find.byType(Table));
      return tester.getRect(find.byKey(key)).shift(-origin);
    }

    await tester.pumpWidget(host(table(TableCellVerticalAlignment.fill)));
    final retained = tester.renderObject(find.byKey(first));
    expect(local(first), const Rect.fromLTWH(200, 0, 100, 40));
    expect(local(second), const Rect.fromLTWH(100, 0, 100, 40));
    expect(local(third), const Rect.fromLTWH(0, 30, 100, 10));

    await tester.pumpWidget(
      host(table(TableCellVerticalAlignment.intrinsicHeight)),
    );
    expect(tester.renderObject(find.byKey(first)), same(retained));
    expect(local(first), const Rect.fromLTWH(200, 0, 100, 100));
    expect(local(third), const Rect.fromLTWH(0, 90, 100, 10));

    await tester.pumpWidget(
      host(
        Table(
          defaultVerticalAlignment: TableCellVerticalAlignment.fill,
          children: const [
            TableRow(children: [SizedBox(key: first, height: 100)]),
          ],
        ),
      ),
    );
    expect(local(first).height, 0);
  });
}
