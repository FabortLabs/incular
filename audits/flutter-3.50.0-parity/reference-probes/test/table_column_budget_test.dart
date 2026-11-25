import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Table sizes columns from constraints, not flex cell content', (
    tester,
  ) async {
    Future<List<double>> measure({
      required double budget,
      required List<double> naturalWidths,
      Map<int, TableColumnWidth> columns = const {},
      bool tight = false,
      bool unbounded = false,
    }) async {
      final keys = List.generate(naturalWidths.length, (i) => ValueKey(i));
      final table = Table(
        columnWidths: columns,
        children: [
          TableRow(
            children: [
              for (var i = 0; i < naturalWidths.length; i++)
                SizedBox(key: keys[i], width: naturalWidths[i], height: 10),
            ],
          ),
        ],
      );
      final Widget constraint = unbounded
          ? UnconstrainedBox(constrainedAxis: Axis.vertical, child: table)
          : tight
          ? SizedBox(width: budget, child: table)
          : ConstrainedBox(
              constraints: BoxConstraints(maxWidth: budget),
              child: table,
            );
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(child: constraint),
        ),
      );
      expect(tester.takeException(), isNull);
      return [for (final key in keys) tester.getSize(find.byKey(key)).width];
    }

    expect(await measure(budget: 100, naturalWidths: [80, 80]), [50, 50]);
    expect(
      await measure(
        budget: 160,
        naturalWidths: [5, 125, 10],
        columns: const {0: FixedColumnWidth(40), 2: FlexColumnWidth(2)},
      ),
      [40, 40, 80],
    );
    expect(
      await measure(
        budget: 100,
        naturalWidths: [80, 80],
        columns: const {0: FixedColumnWidth(10), 1: FixedColumnWidth(20)},
        tight: true,
      ),
      [45, 55],
    );
    expect(
      await measure(
        budget: 0,
        naturalWidths: [80, 80],
        columns: const {1: FractionColumnWidth(0.5)},
        unbounded: true,
      ),
      [0, 0],
    );
  });
}
