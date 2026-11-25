import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Table keeps labeled cells attached as direction changes', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final table = Table(
      children: const [
        TableRow(
          children: [
            TableCell(child: Text('first')),
            TableCell(child: Text('second')),
          ],
        ),
      ],
    );
    Widget scene(TextDirection direction) =>
        Directionality(textDirection: direction, child: table);
    List<SemanticsNode> children(SemanticsNode node) {
      final out = <SemanticsNode>[];
      node.visitChildren((child) {
        out.add(child);
        return true;
      });
      return out;
    }

    (int, Map<String, int>) ids() {
      final root = tester.getSemantics(find.byType(Table));
      expect(root.role, SemanticsRole.table);
      final row = children(root).single;
      expect(row.role, SemanticsRole.row);
      final cells = children(row);
      expect(cells.length, 2);
      expect(cells.every((node) => node.role == SemanticsRole.cell), isTrue);
      return (row.id, {for (final cell in cells) cell.label: cell.id});
    }

    await tester.pumpWidget(scene(TextDirection.ltr));
    final initial = ids();
    expect(tester.getTopLeft(find.text('first')).dx, 0);
    await tester.pumpWidget(scene(TextDirection.rtl));
    final result = ids();
    expect(tester.getTopLeft(find.text('second')).dx, 0);
    expect(result.$1, initial.$1);
    expect(result.$2, initial.$2);
    handle.dispose();
  });
}
