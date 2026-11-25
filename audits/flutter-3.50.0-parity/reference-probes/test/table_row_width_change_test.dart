import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('resizing columns retains cells within surviving rows', (
    tester,
  ) async {
    Widget form(int columns, int rows, int generation) => Directionality(
      textDirection: TextDirection.ltr,
      child: Table(
        children: List.generate(
          rows,
          (row) => TableRow(
            children: List.generate(
              columns,
              (column) => Text('$generation.$row.$column'),
            ),
          ),
        ),
      ),
    );
    List<RenderObject> renders(int columns, int rows, int generation) => [
      for (var row = 0; row < rows; row++)
        for (var column = 0; column < columns; column++)
          tester.renderObject(find.text('$generation.$row.$column')),
    ];

    await tester.pumpWidget(form(3, 3, 0));
    final old = renders(3, 3, 0);
    await tester.pumpWidget(form(4, 2, 1));
    final widened = renders(4, 2, 1);
    expect(widened.sublist(0, 3), old.sublist(0, 3));
    expect(widened.sublist(4, 7), old.sublist(3, 6));
    expect(old.contains(widened[3]), isFalse);
    expect(old.contains(widened[7]), isFalse);

    await tester.pumpWidget(form(2, 4, 2));
    final narrowed = renders(2, 4, 2);
    expect(narrowed.sublist(0, 2), widened.sublist(0, 2));
    expect(narrowed.sublist(2, 4), widened.sublist(4, 6));
    expect(narrowed.skip(4).any(widened.contains), isFalse);
    expect(narrowed.skip(4).any(old.contains), isFalse);
  });
}
