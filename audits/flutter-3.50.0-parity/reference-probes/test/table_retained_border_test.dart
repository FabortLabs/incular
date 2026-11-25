import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Table paints only its requested strokes, over its cells', (
    tester,
  ) async {
    const cell = Color(0xff282828);
    const blue = Color(0xff0000ff);
    const red = Color(0xffff0000);
    const green = Color(0xff00ff00);
    const border = TableBorder(
      verticalInside: BorderSide(color: blue, width: 2),
      horizontalInside: BorderSide(color: red, width: 3),
      top: BorderSide(color: green, width: 4),
    );
    Widget scene(TextDirection direction, TableBorder? border) =>
        Directionality(
          textDirection: direction,
          child: Center(
            child: SizedBox(
              width: 100,
              height: 80,
              child: Table(
                border: border,
                columnWidths: const {
                  0: FixedColumnWidth(60),
                  1: FixedColumnWidth(40),
                },
                children: const [
                  TableRow(
                    children: [
                      ColoredBox(color: cell, child: SizedBox(height: 20)),
                      ColoredBox(color: cell, child: SizedBox(height: 20)),
                    ],
                  ),
                  TableRow(
                    children: [
                      ColoredBox(color: cell, child: SizedBox(height: 10)),
                      ColoredBox(color: cell, child: SizedBox(height: 10)),
                    ],
                  ),
                ],
              ),
            ),
          ),
        );
    RenderTable render() => tester.renderObject(find.byType(Table));
    await tester.pumpWidget(scene(TextDirection.ltr, null));
    final original = render();
    expect(render(), paints..rect(color: cell));
    expect(render(), isNot(paints..path()));

    for (final direction in TextDirection.values) {
      await tester.pumpWidget(scene(direction, border));
      expect(render(), same(original));
      expect(
        render(),
        paints
          ..rect(color: cell)
          ..path(color: blue, strokeWidth: 2)
          ..path(color: red, strokeWidth: 3)
          ..path(color: green),
      );
    }

    await tester.pumpWidget(scene(TextDirection.rtl, const TableBorder()));
    expect(render(), same(original));
    expect(render(), isNot(paints..path()));
  });
}
